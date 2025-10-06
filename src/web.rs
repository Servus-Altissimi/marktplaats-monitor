use crate::{Configuratie, Monitor, Advertentie};
use std::sync::{Arc, Mutex};
use std::fs;
use std::io::{BufRead, BufReader};
use warp::{Filter, Reply};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize)]
struct ResultaatArtikel {
    tijdstempel: String,
    zoekwoord: String,
    titel: String,
    prijs: String,
    locatie: String,
    afstand: String,
    link: String,
    beschrijving: String,
    afbeelding: Option<String>,
}

#[derive(Debug, Serialize)]
struct StatusBericht {
    status: String,
    bericht: String,
}

#[derive(Debug, Deserialize)]
struct MarkeerGezienVerzoek {
    links: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ZoekQuery {
    q: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConfigUpdate {
    postcode: String,
    afstand_km: u32,
    check_interval_seconden: u64,
    max_advertenties_per_zoekopdracht: u32,
    toon_bieden: bool,
    toon_gratis: bool,
    toon_zie_beschrijving: bool,
}

#[derive(Debug, Deserialize)]
struct WenslijstUpdate {
    artikelen: Vec<WenslijstArtikelInput>,
}

#[derive(Debug, Deserialize)]
struct WenslijstArtikelInput {
    zoekwoord: String,
    max_prijs: String,
}

pub async fn start_web_server(poort: u16, config: Arc<Mutex<Configuratie>>, monitor: Arc<Mutex<Monitor>>) {
    let config_filter = warp::any().map(move || config.clone());
    let monitor_filter = warp::any().map(move || monitor.clone());

    let index = warp::get()
        .and(warp::path::end())
        .map(|| {
            let html = index_html();
            warp::reply::html(html)
        });

    let resultaten = warp::get()
        .and(warp::path("resultaten"))
        .and(warp::query::<ZoekQuery>())
        .and(config_filter.clone())
        .and_then(haal_resultaten);

    let config_get = warp::get()
        .and(warp::path("config"))
        .and(config_filter.clone())
        .and_then(haal_config);

    let config_post = warp::post()
        .and(warp::path("config"))
        .and(warp::body::json())
        .and(config_filter.clone())
        .and_then(update_config);

    let zoek = warp::post()
        .and(warp::path("zoek"))
        .and(warp::body::json())
        .and(config_filter.clone())
        .and_then(zoek_nu);

    let wenslijst_get = warp::get()
        .and(warp::path("wenslijst"))
        .and(config_filter.clone())
        .and_then(haal_wenslijst);

    let wenslijst_post = warp::post()
        .and(warp::path("wenslijst"))
        .and(warp::body::json())
        .and(config_filter.clone())
        .and_then(update_wenslijst);

    let markeer_gezien = warp::post()
        .and(warp::path("markeer_gezien"))
        .and(warp::body::json())
        .and(config_filter.clone())
        .and_then(markeer_als_gezien);

    let wis_resultaten = warp::post()
        .and(warp::path("wis_resultaten"))
        .and(config_filter.clone())
        .and(monitor_filter.clone())
        .and_then(wis_alle_resultaten);

    let routes = index
        .or(resultaten)
        .or(config_get)
        .or(config_post)
        .or(zoek)
        .or(wenslijst_get)
        .or(wenslijst_post)
        .or(markeer_gezien)
        .or(wis_resultaten);

    println!("Web interface draait op http://localhost:{}", poort);
    warp::serve(routes).run(([127, 0, 0, 1], poort)).await;
}

async fn markeer_als_gezien(verzoek: MarkeerGezienVerzoek, config: Arc<Mutex<Configuratie>>) -> Result<impl Reply, warp::Rejection> {
    let configuratie = config.lock().unwrap();
    let gezien_bestand = "gezien.txt";
    
    if let Ok(mut bestand) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(gezien_bestand) {
        
        use std::io::Write;
        for link in verzoek.links {
            writeln!(bestand, "{}", link).ok();
        }
    }
    
    Ok(warp::reply::json(&StatusBericht {
        status: "ok".to_string(),
        bericht: "Artikelen gemarkeerd als gezien".to_string(),
    }))
}

async fn wis_alle_resultaten(config: Arc<Mutex<Configuratie>>, monitor: Arc<Mutex<Monitor>>) -> Result<impl Reply, warp::Rejection> {
    let configuratie = config.lock().unwrap();
    let bestand_pad = &configuratie.resultaten_bestand;
    
    if let Err(_) = fs::write(bestand_pad, "") {
        return Ok(warp::reply::json(&StatusBericht {
            status: "error".to_string(),
            bericht: "Kon resultaten niet wissen".to_string(),
        }));
    }
    
    let gezien_bestand = "gezien.txt";
    if Path::new(gezien_bestand).exists() {
        fs::remove_file(gezien_bestand).ok();
    }
    
    let mut monitor_lock = monitor.lock().unwrap();
    monitor_lock.gezien_advertenties.clear();
    
    Ok(warp::reply::json(&StatusBericht {
        status: "ok".to_string(),
        bericht: "Alle resultaten en gezien artikelen permanent gewist".to_string(),
    }))
}

async fn haal_resultaten(query: ZoekQuery, config: Arc<Mutex<Configuratie>>) -> Result<impl Reply, warp::Rejection> {
    let configuratie = config.lock().unwrap();
    let bestand_pad = &configuratie.resultaten_bestand;
    
    let mut resultaten = Vec::new();
    
    if let Ok(bestand) = fs::File::open(bestand_pad) {
        let lezer = BufReader::new(bestand);
        let mut huidig_artikel: Option<ResultaatArtikel> = None;
        
        for lijn in lezer.lines() {
            if let Ok(lijn) = lijn {
                if lijn.starts_with("[") && lijn.contains("] Gevonden:") {
                    if let Some(artikel) = huidig_artikel.take() {
                        resultaten.push(artikel);
                    }
                    
                    let onderdelen: Vec<&str> = lijn.splitn(2, "] Gevonden: ").collect();
                    if onderdelen.len() == 2 {
                        let tijdstempel = onderdelen[0].trim_start_matches('[').to_string();
                        let rest = onderdelen[1];
                        let zoekwoord_onderdelen: Vec<&str> = rest.splitn(2, "' (max €").collect();
                        let zoekwoord = zoekwoord_onderdelen[0].trim_start_matches('\'').to_string();
                        
                        huidig_artikel = Some(ResultaatArtikel {
                            tijdstempel,
                            zoekwoord,
                            titel: String::new(),
                            prijs: String::new(),
                            locatie: String::new(),
                            afstand: String::new(),
                            link: String::new(),
                            beschrijving: String::new(),
                            afbeelding: None,
                        });
                    }
                } else if lijn.contains("Titel: ") {
                    if let Some(ref mut artikel) = huidig_artikel {
                        artikel.titel = lijn.trim().trim_start_matches("Titel: ").to_string();
                    }
                } else if lijn.contains("Prijs: ") {
                    if let Some(ref mut artikel) = huidig_artikel {
                        artikel.prijs = lijn.trim().trim_start_matches("Prijs: ").to_string();
                    }
                } else if lijn.contains("Locatie: ") {
                    if let Some(ref mut artikel) = huidig_artikel {
                        let locatie_str = lijn.trim().trim_start_matches("Locatie: ");
                        if let Some(pos) = locatie_str.rfind(" (") {
                            artikel.locatie = locatie_str[..pos].to_string();
                            artikel.afstand = locatie_str[pos+2..].trim_end_matches(')').to_string();
                        } else {
                            artikel.locatie = locatie_str.to_string();
                        }
                    }
                } else if lijn.contains("Link: ") {
                    if let Some(ref mut artikel) = huidig_artikel {
                        artikel.link = lijn.trim().trim_start_matches("Link: ").to_string();
                    }
                } else if lijn.contains("Afbeelding: ") {
                    if let Some(ref mut artikel) = huidig_artikel {
                        let afb = lijn.trim().trim_start_matches("Afbeelding: ").to_string();
                        if afb != "Geen afbeelding" && !afb.is_empty() {
                            artikel.afbeelding = Some(afb);
                        }
                    }
                } else if lijn.contains("Beschrijving: ") {
                    if let Some(ref mut artikel) = huidig_artikel {
                        artikel.beschrijving = lijn.trim().trim_start_matches("Beschrijving: ").to_string();
                    }
                }
            }
        }
        
        if let Some(artikel) = huidig_artikel {
            resultaten.push(artikel);
        }
    }
    
    if let Some(zoekterm) = query.q {
        let zoekterm_lower = zoekterm.to_lowercase();
        resultaten.retain(|r| {
            r.titel.to_lowercase().contains(&zoekterm_lower) ||
            r.beschrijving.to_lowercase().contains(&zoekterm_lower) ||
            r.zoekwoord.to_lowercase().contains(&zoekterm_lower)
        });
    }
    
    resultaten.reverse();
    
    Ok(warp::reply::json(&resultaten))
}

async fn haal_config(config: Arc<Mutex<Configuratie>>) -> Result<impl Reply, warp::Rejection> {
    let configuratie = config.lock().unwrap();
    Ok(warp::reply::json(&*configuratie))
}

async fn update_config(update: ConfigUpdate, config: Arc<Mutex<Configuratie>>) -> Result<impl Reply, warp::Rejection> {
    let mut configuratie = config.lock().unwrap();
    
    configuratie.postcode = update.postcode;
    configuratie.afstand_km = update.afstand_km;
    configuratie.check_interval_seconden = update.check_interval_seconden;
    configuratie.max_advertenties_per_zoekopdracht = update.max_advertenties_per_zoekopdracht;
    configuratie.toon_bieden = update.toon_bieden;
    configuratie.toon_gratis = update.toon_gratis;
    configuratie.toon_zie_beschrijving = update.toon_zie_beschrijving;
    
    let toml_string = toml::to_string_pretty(&*configuratie).unwrap();
    fs::write("config.toml", toml_string).ok();
    
    Ok(warp::reply::json(&StatusBericht {
        status: "ok".to_string(),
        bericht: "Configuratie opgeslagen".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
struct ZoekVerzoek {
    zoekwoord: String,
}

async fn zoek_nu(verzoek: ZoekVerzoek, config: Arc<Mutex<Configuratie>>) -> Result<impl Reply, warp::Rejection> {
    let configuratie = config.lock().unwrap().clone();
    
    let monitor = Monitor::nieuw(configuratie).ok();
    
    if let Some(monitor) = monitor {
        match monitor.zoek_artikel(&verzoek.zoekwoord, i32::MAX).await {
            Ok(advertenties) => {
                Ok(warp::reply::json(&advertenties))
            },
            Err(_) => {
                Ok(warp::reply::json(&Vec::<Advertentie>::new()))
            }
        }
    } else {
        Ok(warp::reply::json(&Vec::<Advertentie>::new()))
    }
}

async fn haal_wenslijst(config: Arc<Mutex<Configuratie>>) -> Result<impl Reply, warp::Rejection> {
    let configuratie = config.lock().unwrap();
    let bestand_pad = &configuratie.wenslijst_bestand;
    
    let mut artikelen = Vec::new();
    
    if let Ok(bestand) = fs::File::open(bestand_pad) {
        let lezer = BufReader::new(bestand);
        
        for lijn in lezer.lines() {
            if let Ok(lijn) = lijn {
                let lijn = lijn.trim();
                
                if lijn.is_empty() || lijn.starts_with("#") {
                    continue;
                }
                
                if lijn.contains(";") {
                    let onderdelen: Vec<&str> = lijn.splitn(2, ";").collect();
                    if onderdelen.len() == 2 {
                        artikelen.push(serde_json::json!({
                            "zoekwoord": onderdelen[0].trim(),
                            "max_prijs": onderdelen[1].trim()
                        }));
                    }
                }
            }
        }
    }
    
    Ok(warp::reply::json(&artikelen))
}

async fn update_wenslijst(update: WenslijstUpdate, config: Arc<Mutex<Configuratie>>) -> Result<impl Reply, warp::Rejection> {
    let configuratie = config.lock().unwrap();
    let bestand_pad = &configuratie.wenslijst_bestand;
    
    let mut inhoud = String::from("# Marktplaats Wensenlijst\n");
    inhoud.push_str("# Formaat: zoekwoord;maximaleprijs\n");
    inhoud.push_str("# Om te commenteren gebruikt u #\n");
    inhoud.push_str("# Als u geen maximale prijs wilt, stelt u de prijs in als -1\n");
    inhoud.push_str("# Wilt u gratis producten, doe 0 als de prijs\n\n");
    
    for artikel in update.artikelen {
        inhoud.push_str(&format!("{};{}\n", artikel.zoekwoord, artikel.max_prijs));
    }
    
    fs::write(bestand_pad, inhoud).ok();
    
    Ok(warp::reply::json(&StatusBericht {
        status: "ok".to_string(),
        bericht: "Wenslijst opgeslagen".to_string(),
    }))
}



fn index_html() -> String {
    r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Marktplaats Monitor</title>
    <style>
        body { font-family: Arial; margin: 20px; background: #f5f5f5; min-height: 100vh; display: flex; flex-direction: column; }
        .content { flex: 1; }
        h1 { color: #333; }
        .status-bericht { padding: 10px; margin: 10px 0; border-radius: 5px; display: none; }
        .status-bericht.success { background: #d4edda; color: #155724; border: 1px solid #c3e6cb; }
        .status-bericht.error { background: #f8d7da; color: #721c24; border: 1px solid #f5c6cb; }
        .zoekbalk { margin: 20px 0; }
        input[type="text"] { padding: 8px; width: 300px; }
        button { padding: 8px 16px; background: rgb(255, 143, 68); color: white; border: none; cursor: pointer; margin-right: 5px; }
        button:hover { background: #0056b3; }
        button.danger { background: #dc3545; }
        button.danger:hover { background: #c82333; }
        .resultaat { background: white; padding: 15px; margin: 10px 0; border: 1px solid #ddd; border-radius: 5px; position: relative; }
        .resultaat.nieuw { border-left: 4px solid #28a745; }
        .resultaat img { max-width: 150px; max-height: 150px; float: left; margin-right: 15px; border-radius: 5px; object-fit: cover; }
        .resultaat h3 { margin: 0 0 10px 0; }
        .resultaat a { color: #007bff; text-decoration: none; }
        .resultaat a:hover { text-decoration: underline; }
        .prijs { font-weight: bold; color: rgb(0, 190, 44); }
        .info { color: #666; font-size: 14px; }
        .nieuw-stempel { background: #28a745; color: white; padding: 3px 2px; border-radius: 3px; font-size: 12px; position: absolute; top: 4px; right: 10px; }
        .markeer-gezien-btn { float: right; padding: 5px 10px; font-size: 12px; }
        .tabs { margin: 20px 0; border-bottom: 2px solid #ddd; }
        .tab { display: inline-block; padding: 10px 20px; cursor: pointer; background: #e9ecef; margin-right: 5px; }
        .tab.active { background: white; border: 1px solid #ddd; border-bottom: none; }
        .tab-content { display: none; }
        .tab-content.active { display: block; }
        .config-form { background: white; padding: 20px; max-width: 600px; }
        .config-form label { display: block; margin: 10px 0 5px 0; }
        .config-form input[type="text"], .config-form input[type="number"] { padding: 8px; width: 100%; box-sizing: border-box; }
        .config-form input[type="checkbox"] { margin-right: 5px; }
        .wenslijst-artikel { background: #f9f9f9; padding: 10px; margin: 10px 0; border: 1px solid #ddd; border-radius: 5px; display: flex; gap: 10px; align-items: center; }
        .wenslijst-artikel input { flex: 1; padding: 8px; }
        .wenslijst-artikel button { padding: 5px 10px; background: #dc3545; }
        .wenslijst-artikel button:hover { background: #c82333; }
        footer { margin-top: 40px; padding: 20px; text-align: center; color: black; border-radius: 5px; }
        footer a { color: #4db8ff; text-decoration: none; }
        footer a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <div class="content">
        <h1>Marktplaats Monitor</h1>
        
        <div id="status-bericht" class="status-bericht"></div>
        
        <div class="tabs">
            <div class="tab active" onclick="toonTab('nieuwe-artikelen')">Nieuwe Artikelen</div>
            <div class="tab" onclick="toonTab('resultaten')">Alle Resultaten</div>
            <div class="tab" onclick="toonTab('config')">Configuratie</div>
            <div class="tab" onclick="toonTab('wenslijst')">Wenslijst</div>
        </div>
        
        <div id="nieuwe-artikelen-tab" class="tab-content active">
            <div style="margin: 20px 0;">
                <button onclick="markeerAlleNieuweAlsGezien()">Markeer alle als gezien</button>
            </div>
            <div id="nieuwe-artikelen"></div>
        </div>
        
        <div id="resultaten-tab" class="tab-content">
            <div class="zoekbalk">
                <input type="text" id="zoekterm" placeholder="Zoek in resultaten...">
                <button onclick="zoekResultaten()">Zoeken</button>
                <button onclick="laadResultaten()">Alles tonen</button>
             <!--   <button class="danger" onclick="wisAlleResultaten()">Alle Artikelen wissen</button>  -->
            </div>
            <div id="resultaten"></div>
        </div>
        
        <div id="config-tab" class="tab-content">
            <div class="config-form">
                <h2>Configuratie</h2>
                <label>Postcode:</label>
                <input type="text" id="postcode">
                
                <label>Afstand (km):</label>
                <input type="number" id="afstand_km">
                
                <label>Check interval (seconden):</label>
                <input type="number" id="check_interval">
                
                <label>Max advertenties per zoekopdracht:</label>
                <input type="number" id="max_advertenties">
                
                <label><input type="checkbox" id="toon_bieden"> Toon bieden</label>
                <label><input type="checkbox" id="toon_gratis"> Toon gratis</label>
                <label><input type="checkbox" id="toon_zie_beschrijving"> Toon zie beschrijving</label>
                
                <br><br>
                <button onclick="bewaarConfig()">Opslaan</button>
               <!-- <button class="danger" onclick="herstartProgramma()">Herstart Programma</button> -->
            </div>
        </div>
        
        <div id="wenslijst-tab" class="tab-content">
            <div class="config-form">
                <h2>Wenslijst</h2>
                <div id="wenslijst-artikelen"></div>
                <br>
                <button onclick="voegWenslijstArtikelToe()">+ Nieuw artikel toevoegen</button>
                <button onclick="bewaarWenslijst()">Opslaan</button>
            </div>
        </div>
    </div>
    
    <footer>
        <p>Een product van <a href="https://constringo.com" target="_blank">Constringo</a>, neem contact!</p>
    </footer>
    
    <script>
        let wenslijstArtikelen = [];
        let gezienArtikelen = new Set();
        
        function toonStatusBericht(bericht, isSuccess) {
            const element = document.getElementById('status-bericht');
            element.textContent = bericht;
            element.className = 'status-bericht ' + (isSuccess ? 'success' : 'error');
            element.style.display = 'block';
            
            setTimeout(() => {
                element.style.display = 'none';
            }, 3000);
        }
        
        function laad_gezien_artikelen() {
            const opgeslagen = localStorage.getItem('gezien_artikelen');
            if (opgeslagen) {
                gezienArtikelen = new Set(JSON.parse(opgeslagen));
            }
        }
        
        function bewaar_gezien_artikelen() {
            localStorage.setItem('gezien_artikelen', JSON.stringify([...gezienArtikelen]));
        }
        
        function toonTab(tab) {
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            
            event.target.classList.add('active');
            document.getElementById(tab + '-tab').classList.add('active');
            
            if (tab === 'config') {
                laadConfig();
            } else if (tab === 'wenslijst') {
                laadWenslijst();
            } else if (tab === 'nieuwe-artikelen') {
                laadNieuweArtikelen();
            } else {
                laadResultaten();
            }
        }
        
        function laadNieuweArtikelen() {
            fetch('/resultaten')
                .then(r => r.json())
                .then(data => {
                    const container = document.getElementById('nieuwe-artikelen');
                    container.innerHTML = '';
                    
                    const nieuweArtikelen = data.filter(artikel => !gezienArtikelen.has(artikel.link));
                    
                    if (nieuweArtikelen.length === 0) {
                        container.innerHTML = '<p>Geen nieuwe artikelen.</p>';
                        return;
                    }
                    
                    nieuweArtikelen.forEach(artikel => {
                        const div = document.createElement('div');
                        div.className = 'resultaat nieuw';
                        div.dataset.link = artikel.link;
                        
                        let afbeelding = '';
                        if (artikel.afbeelding) {
                            afbeelding = `<img src="${artikel.afbeelding}" alt="${artikel.titel}">`;
                        }
                        
                        div.innerHTML = `
                            <span class="nieuw-stempel">NIEUW</span>
                            <button class="markeer-gezien-btn" onclick="markeerAlsGezien('${artikel.link}')">Gezien</button>
                            ${afbeelding}
                            <h3><a href="${artikel.link}" target="_blank">${artikel.titel}</a></h3>
                            <div class="prijs">${artikel.prijs}</div>
                            <div class="info">
                                Locatie: ${artikel.locatie} (${artikel.afstand})<br>
                                Zoekwoord: ${artikel.zoekwoord}<br>
                                ${artikel.tijdstempel}
                            </div>
                            <p>${artikel.beschrijving}</p>
                            <div style="clear: both;"></div>
                        `;
                        
                        container.appendChild(div);
                    });
                });
        }
        
        function markeerAlsGezien(link) {
            gezienArtikelen.add(link);
            bewaar_gezien_artikelen();
            
            fetch('/markeer_gezien', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ links: [link] })
            });
            
            laadNieuweArtikelen();
        }
        
        function markeerAlleNieuweAlsGezien() {
            fetch('/resultaten')
                .then(r => r.json())
                .then(data => {
                    const nieuweLinks = data
                        .filter(artikel => !gezienArtikelen.has(artikel.link))
                        .map(artikel => artikel.link);
                    
                    if (nieuweLinks.length === 0) {
                        toonStatusBericht('Geen nieuwe artikelen om te markeren', false);
                        return;
                    }
                    
                    nieuweLinks.forEach(link => gezienArtikelen.add(link));
                    bewaar_gezien_artikelen();
                    
                    fetch('/markeer_gezien', {
                        method: 'POST',
                        headers: { 'Content-Type': 'application/json' },
                        body: JSON.stringify({ links: nieuweLinks })
                    })
                    .then(() => {
                        toonStatusBericht(`${nieuweLinks.length} artikelen gemarkeerd als gezien`, true);
                        laadNieuweArtikelen();
                    });
                });
        }

        function wisAlleResultaten() {
            fetch('/wis_resultaten', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' }
            })
            .then(r => r.json())
            .then(data => {
                if (data.status === 'ok') {
                    localStorage.removeItem('gezien_artikelen');
                    gezienArtikelen.clear();
                    
                    toonStatusBericht(data.bericht, true);
                    laadNieuweArtikelen();
                    laadResultaten();
                } else {
                    toonStatusBericht(data.bericht, false);
                }
            })
            .catch(err => {
                toonStatusBericht('Er is iets missgegaan tijdens het wissen: ' + err, false);
            });
        }
        
        function laadResultaten() {
            fetch('/resultaten')
                .then(r => r.json())
                .then(data => {
                    const container = document.getElementById('resultaten');
                    container.innerHTML = '';
                    
                    if (data.length === 0) {
                        container.innerHTML = '<p>Geen resultaten gevonden.</p>';
                        return;
                    }
                    
                    data.forEach(artikel => {
                        const div = document.createElement('div');
                        div.className = 'resultaat';
                        
                        let afbeelding = '';
                        if (artikel.afbeelding) {
                            afbeelding = `<img src="${artikel.afbeelding}" alt="${artikel.titel}">`;
                        }
                        
                        div.innerHTML = `
                            ${afbeelding}
                            <h3><a href="${artikel.link}" target="_blank">${artikel.titel}</a></h3>
                            <div class="prijs">${artikel.prijs}</div>
                            <div class="info">
                                Locatie: ${artikel.locatie} (${artikel.afstand})<br>
                                Zoekwoord: ${artikel.zoekwoord}<br>
                                ${artikel.tijdstempel}
                                </div>
                            <p>${artikel.beschrijving}</p>
                            <div style="clear: both;"></div>
                        `;
                        
                        container.appendChild(div);
                    });
                });
        }
        
        function zoekResultaten() {
            const zoekterm = document.getElementById('zoekterm').value;
            fetch('/resultaten?q=' + encodeURIComponent(zoekterm))
                .then(r => r.json())
                .then(data => {
                    const container = document.getElementById('resultaten');
                    container.innerHTML = '';
                    
                    if (data.length === 0) {
                        container.innerHTML = '<p>Geen resultaten gevonden.</p>';
                        return;
                    }
                    
                    data.forEach(artikel => {
                        const div = document.createElement('div');
                        div.className = 'resultaat';
                        
                        let afbeelding = '';
                        if (artikel.afbeelding) {
                            afbeelding = `<img src="${artikel.afbeelding}" alt="${artikel.titel}">`;
                        }
                        
                        div.innerHTML = `
                            ${afbeelding}
                            <h3><a href="${artikel.link}" target="_blank">${artikel.titel}</a></h3>
                            <div class="prijs">${artikel.prijs}</div>
                            <div class="info">
                                Locatie: ${artikel.locatie} (${artikel.afstand})<br>
                                Zoekwoord: ${artikel.zoekwoord}<br>
                                ${artikel.tijdstempel}
                            </div>
                            <p>${artikel.beschrijving}</p>
                            <div style="clear: both;"></div>
                        `;
                        
                        container.appendChild(div);
                    });
                });
        }
        
        function laadConfig() {
            fetch('/config')
                .then(r => r.json())
                .then(data => {
                    document.getElementById('postcode').value = data.postcode;
                    document.getElementById('afstand_km').value = data.afstand_km;
                    document.getElementById('check_interval').value = data.check_interval_seconden;
                    document.getElementById('max_advertenties').value = data.max_advertenties_per_zoekopdracht;
                    document.getElementById('toon_bieden').checked = data.toon_bieden;
                    document.getElementById('toon_gratis').checked = data.toon_gratis;
                    document.getElementById('toon_zie_beschrijving').checked = data.toon_zie_beschrijving;
                });
        }
        
        function bewaarConfig() {
            const config = {
                postcode: document.getElementById('postcode').value,
                afstand_km: parseInt(document.getElementById('afstand_km').value),
                check_interval_seconden: parseInt(document.getElementById('check_interval').value),
                max_advertenties_per_zoekopdracht: parseInt(document.getElementById('max_advertenties').value),
                toon_bieden: document.getElementById('toon_bieden').checked,
                toon_gratis: document.getElementById('toon_gratis').checked,
                toon_zie_beschrijving: document.getElementById('toon_zie_beschrijving').checked
            };
            
            fetch('/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(config)
            })
            .then(r => r.json())
            .then(data => {
                toonStatusBericht(data.bericht, data.status === 'ok');
            });
        }
        
        function laadWenslijst() {
            fetch('/wenslijst')
                .then(r => r.json())
                .then(data => {
                    wenslijstArtikelen = data;
                    toonWenslijstArtikelen();
                });
        }
        
        function toonWenslijstArtikelen() {
            const container = document.getElementById('wenslijst-artikelen');
            container.innerHTML = '';
            
            wenslijstArtikelen.forEach((artikel, index) => {
                const div = document.createElement('div');
                div.className = 'wenslijst-artikel';
                div.innerHTML = `
                    <input type="text" value="${artikel.zoekwoord}" onchange="updateWenslijstArtikel(${index}, 'zoekwoord', this.value)" placeholder="Zoekwoord">
                    <input type="text" value="${artikel.max_prijs}" onchange="updateWenslijstArtikel(${index}, 'max_prijs', this.value)" placeholder="Max prijs (-1 = onbeperkt, 0 = gratis)">
                    <button onclick="verwijderWenslijstArtikel(${index})">Verwijderen</button>
                `;
                container.appendChild(div);
            });
        }
        
        function updateWenslijstArtikel(index, veld, waarde) {
            wenslijstArtikelen[index][veld] = waarde;
        }
        
        function voegWenslijstArtikelToe() {
            wenslijstArtikelen.push({ zoekwoord: '', max_prijs: '-1' });
            toonWenslijstArtikelen();
        }
        
        function verwijderWenslijstArtikel(index) {
            wenslijstArtikelen.splice(index, 1);
            toonWenslijstArtikelen();
        }
        
        function bewaarWenslijst() {
            fetch('/wenslijst', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ artikelen: wenslijstArtikelen })
            })
            .then(r => r.json())
            .then(data => {
                toonStatusBericht(data.bericht, data.status === 'ok');
            });
        }
        
        laad_gezien_artikelen();
        laadNieuweArtikelen();
    </script>
</body>
</html>"#.to_string()
}