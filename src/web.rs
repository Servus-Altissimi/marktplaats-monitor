use crate::{Configuratie, Monitor, Advertentie};
use std::sync::{Arc, Mutex};
use std::fs;
use std::io::{BufRead, BufReader};
use warp::{Filter, Reply};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ResultaatItem {
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
    items: Vec<WenslijstItemInput>,
}

#[derive(Debug, Deserialize)]
struct WenslijstItemInput {
    zoekwoord: String,
    max_prijs: String,
}

pub async fn start_web_server(poort: u16, config: Arc<Mutex<Configuratie>>) {
    let config_filter = warp::any().map(move || config.clone());

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

    let routes = index
        .or(resultaten)
        .or(config_get)
        .or(config_post)
        .or(zoek)
        .or(wenslijst_get)
        .or(wenslijst_post);

    println!("Web interface draait op http://localhost:{}", poort);
    warp::serve(routes).run(([127, 0, 0, 1], poort)).await;
}

async fn haal_resultaten(query: ZoekQuery, config: Arc<Mutex<Configuratie>>) -> Result<impl Reply, warp::Rejection> {
    let configuratie = config.lock().unwrap();
    let bestand_pad = &configuratie.resultaten_bestand;
    
    let mut resultaten = Vec::new();
    
    if let Ok(bestand) = fs::File::open(bestand_pad) {
        let lezer = BufReader::new(bestand);
        let mut huidige_item: Option<ResultaatItem> = None;
        
        for lijn in lezer.lines() {
            if let Ok(lijn) = lijn {
                if lijn.starts_with("[") && lijn.contains("] Gevonden:") {
                    if let Some(item) = huidige_item.take() {
                        resultaten.push(item);
                    }
                    
                    let onderdelen: Vec<&str> = lijn.splitn(2, "] Gevonden: ").collect();
                    if onderdelen.len() == 2 {
                        let tijdstempel = onderdelen[0].trim_start_matches('[').to_string();
                        let rest = onderdelen[1];
                        let zoekwoord_onderdelen: Vec<&str> = rest.splitn(2, "' (max €").collect();
                        let zoekwoord = zoekwoord_onderdelen[0].trim_start_matches('\'').to_string();
                        
                        huidige_item = Some(ResultaatItem {
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
                    if let Some(ref mut item) = huidige_item {
                        item.titel = lijn.trim().trim_start_matches("Titel: ").to_string();
                    }
                } else if lijn.contains("Prijs: ") {
                    if let Some(ref mut item) = huidige_item {
                        item.prijs = lijn.trim().trim_start_matches("Prijs: ").to_string();
                    }
                } else if lijn.contains("Locatie: ") {
                    if let Some(ref mut item) = huidige_item {
                        let locatie_str = lijn.trim().trim_start_matches("Locatie: ");
                        if let Some(pos) = locatie_str.rfind(" (") {
                            item.locatie = locatie_str[..pos].to_string();
                            item.afstand = locatie_str[pos+2..].trim_end_matches(')').to_string();
                        } else {
                            item.locatie = locatie_str.to_string();
                        }
                    }
                } else if lijn.contains("Link: ") {
                    if let Some(ref mut item) = huidige_item {
                        item.link = lijn.trim().trim_start_matches("Link: ").to_string();
                    }
                } else if lijn.contains("Afbeelding: ") {
                    if let Some(ref mut item) = huidige_item {
                        let afb = lijn.trim().trim_start_matches("Afbeelding: ").to_string();
                        if afb != "Geen afbeelding" && !afb.is_empty() {
                            item.afbeelding = Some(afb);
                        }
                    }
                } else if lijn.contains("Beschrijving: ") {
                    if let Some(ref mut item) = huidige_item {
                        item.beschrijving = lijn.trim().trim_start_matches("Beschrijving: ").to_string();
                    }
                }
            }
        }
        
        if let Some(item) = huidige_item {
            resultaten.push(item);
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
    
    Ok(warp::reply::json(&serde_json::json!({"status": "ok"})))
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
    
    let mut items = Vec::new();
    
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
                        items.push(serde_json::json!({
                            "zoekwoord": onderdelen[0].trim(),
                            "max_prijs": onderdelen[1].trim()
                        }));
                    }
                }
            }
        }
    }
    
    Ok(warp::reply::json(&items))
}

async fn update_wenslijst(update: WenslijstUpdate, config: Arc<Mutex<Configuratie>>) -> Result<impl Reply, warp::Rejection> {
    let configuratie = config.lock().unwrap();
    let bestand_pad = &configuratie.wenslijst_bestand;
    
    let mut inhoud = String::from("# Marktplaats Wensenlijst\n");
    inhoud.push_str("# Formaat: zoekwoord;maximaleprijs\n");
    inhoud.push_str("# Om te commenteren gebruikt u #\n");
    inhoud.push_str("# Als u geen maximale prijs wilt, stelt u de prijs in als -1\n");
    inhoud.push_str("# Wilt u gratis producten, doe 0 als de prijs\n\n");
    
    for item in update.items {
        inhoud.push_str(&format!("{};{}\n", item.zoekwoord, item.max_prijs));
    }
    
    fs::write(bestand_pad, inhoud).ok();
    
    Ok(warp::reply::json(&serde_json::json!({"status": "ok"})))
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
        .zoekbalk { margin: 20px 0; }
        input[type="text"] { padding: 8px; width: 300px; }
        button { padding: 8px 16px; background: rgb(255, 143, 68); color: white; border: none; cursor: pointer; }
        button:hover { background: #0056b3; }
        .resultaat { background: white; padding: 15px; margin: 10px 0; border: 1px solid #ddd; border-radius: 5px; }
        .resultaat img { max-width: 150px; max-height: 150px; float: left; margin-right: 15px; border-radius: 5px; object-fit: cover; }
        .resultaat h3 { margin: 0 0 10px 0; }
        .resultaat a { color: #007bff; text-decoration: none; }
        .resultaat a:hover { text-decoration: underline; }
        .prijs { font-weight: bold; color: rgb(0, 190, 44); }
        .info { color: #666; font-size: 14px; }
        .tabs { margin: 20px 0; border-bottom: 2px solid #ddd; }
        .tab { display: inline-block; padding: 10px 20px; cursor: pointer; background: #e9ecef; margin-right: 5px; }
        .tab.active { background: white; border: 1px solid #ddd; border-bottom: none; }
        .tab-content { display: none; }
        .tab-content.active { display: block; }
        .config-form { background: white; padding: 20px; max-width: 600px; }
        .config-form label { display: block; margin: 10px 0 5px 0; }
        .config-form input[type="text"], .config-form input[type="number"] { padding: 8px; width: 100%; box-sizing: border-box; }
        .config-form input[type="checkbox"] { margin-right: 5px; }
        .wenslijst-item { background: #f9f9f9; padding: 10px; margin: 10px 0; border: 1px solid #ddd; border-radius: 5px; display: flex; gap: 10px; align-items: center; }
        .wenslijst-item input { flex: 1; padding: 8px; }
        .wenslijst-item button { padding: 5px 10px; background: #dc3545; }
        .wenslijst-item button:hover { background: #c82333; }
        footer { margin-top: 40px; padding: 20px; text-align: center; color: black; border-radius: 5px; }
        footer a { color: #4db8ff; text-decoration: none; }
        footer a:hover { text-decoration: underline; }
    </style>
</head>
<body>
    <div class="content">
        <h1>Marktplaats Monitor</h1>
        
        <div class="tabs">
            <div class="tab active" onclick="toonTab('resultaten')">Resultaten</div>
            <div class="tab" onclick="toonTab('config')">Configuratie</div>
            <div class="tab" onclick="toonTab('wenslijst')">Wenslijst</div>
        </div>
        
        <div id="resultaten-tab" class="tab-content active">
            <div class="zoekbalk">
                <input type="text" id="zoekterm" placeholder="Zoek in resultaten...">
                <button onclick="zoekResultaten()">Zoeken</button>
                <button onclick="laadResultaten()">Alles tonen</button>
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
            </div>
        </div>
        
        <div id="wenslijst-tab" class="tab-content">
            <div class="config-form">
                <h2>Wenslijst</h2>
                <div id="wenslijst-items"></div>
                <br>
                <button onclick="voegWenslijstItemToe()">+ Nieuw item toevoegen</button>
                <button onclick="bewaarWenslijst()">Opslaan</button>
            </div>
        </div>
    </div>
    
    <footer>
        <p>Een product van <a href="https://constringo.com" target="_blank">Constringo</a>, neem contact!</p>
    </footer>
    
    <script>
        let wenslijstItems = [];
        
        function toonTab(tab) {
            document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
            
            event.target.classList.add('active');
            document.getElementById(tab + '-tab').classList.add('active');
            
            if (tab === 'config') {
                laadConfig();
            } else if (tab === 'wenslijst') {
                laadWenslijst();
            } else {
                laadResultaten();
            }
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
                    
                    data.forEach(item => {
                        const div = document.createElement('div');
                        div.className = 'resultaat';
                        
                        let afbeelding = '';
                        if (item.afbeelding) {
                            afbeelding = `<img src="${item.afbeelding}" alt="${item.titel}">`;
                        }
                        
                        div.innerHTML = `
                            ${afbeelding}
                            <h3><a href="${item.link}" target="_blank">${item.titel}</a></h3>
                            <div class="prijs">${item.prijs}</div>
                            <div class="info">
                                Locatie: ${item.locatie} (${item.afstand})<br>
                                Zoekwoord: ${item.zoekwoord}<br>
                                ${item.tijdstempel}
                            </div>
                            <p>${item.beschrijving}</p>
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
                    
                    data.forEach(item => {
                        const div = document.createElement('div');
                        div.className = 'resultaat';
                        
                        let afbeelding = '';
                        if (item.afbeelding) {
                            afbeelding = `<img src="${item.afbeelding}" alt="${item.titel}">`;
                        }
                        
                        div.innerHTML = `
                            ${afbeelding}
                            <h3><a href="${item.link}" target="_blank">${item.titel}</a></h3>
                            <div class="prijs">${item.prijs}</div>
                            <div class="info">
                                Locatie: ${item.locatie} (${item.afstand})<br>
                                Zoekwoord: ${item.zoekwoord}<br>
                                ${item.tijdstempel}
                            </div>
                            <p>${item.beschrijving}</p>
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
                alert('Configuratie opgeslagen! Herstart het programma voor volledige effect.');
            });
        }
        
        function laadWenslijst() {
            fetch('/wenslijst')
                .then(r => r.json())
                .then(data => {
                    wenslijstItems = data;
                    toonWenslijstItems();
                });
        }
        
        function toonWenslijstItems() {
            const container = document.getElementById('wenslijst-items');
            container.innerHTML = '';
            
            wenslijstItems.forEach((item, index) => {
                const div = document.createElement('div');
                div.className = 'wenslijst-item';
                div.innerHTML = `
                    <input type="text" value="${item.zoekwoord}" onchange="updateWenslijstItem(${index}, 'zoekwoord', this.value)" placeholder="Zoekwoord">
                    <input type="text" value="${item.max_prijs}" onchange="updateWenslijstItem(${index}, 'max_prijs', this.value)" placeholder="Max prijs (-1 = onbeperkt, 0 = gratis)">
                    <button onclick="verwijderWenslijstItem(${index})">Verwijderen</button>
                `;
                container.appendChild(div);
            });
        }
        
        function updateWenslijstItem(index, veld, waarde) {
            wenslijstItems[index][veld] = waarde;
        }
        
        function voegWenslijstItemToe() {
            wenslijstItems.push({ zoekwoord: '', max_prijs: '-1' });
            toonWenslijstItems();
        }
        
        function verwijderWenslijstItem(index) {
            wenslijstItems.splice(index, 1);
            toonWenslijstItems();
        }
        
        function bewaarWenslijst() {
            fetch('/wenslijst', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ items: wenslijstItems })
            })
            .then(r => r.json())
            .then(data => {
                alert('Wenslijst opgeslagen!');
            });
        }
        
        laadResultaten();
    </script>
</body>
</html>"#.to_string()
}