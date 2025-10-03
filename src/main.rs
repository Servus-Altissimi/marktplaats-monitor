// _      ____  ____  _  __ _____ ____  _     ____  ____ _____ ____    _      ____  _      _ _____ ____  ____ 
/// \__/|/  _ \/  __\/ |/ //__ __Y  __\/ \   /  _ \/  _ Y__ __Y ___\  / \__/|/  _ \/ \  /|/ Y__ __Y  _ \/  __\
//| |\/||| / \||  \/||   /   / \ |  \/|| |   | / \|| / \| / \ |    \  | |\/||| / \|| |\ ||| | / \ | / \||  \/|
//| |  ||| |-|||    /|   \   | | |  __/| |_/\| |-||| |-|| | | \___ |  | |  ||| \_/|| | \||| | | | | \_/||    /
//\_/  \|\_/ \|\_/\_\\_|\_\  \_/ \_/   \____/\_/ \|\_/ \| \_/ \____/  \_/  \|\____/\_/  \|\_/ \_/ \____/\_/\_\
                                                                                                            
  
// Dit programma is niet geschreven met slechte bedoelingen.
// Ik haat marktplaats echter, en vindt het een verschrikkelijke dienst.

// Copyright 2025 Servus Altissimi (Pseudonym)

// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

mod web;

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::sync::Arc;
use std::error::Error;
use serde::{Deserialize, Serialize};
use reqwest;
use chrono::Local;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Configuratie {
    pub postcode: String, 
    pub afstand_km: u32,
    pub check_interval_seconden: u64,
    pub max_advertenties_per_zoekopdracht: u32,
    pub wenslijst_bestand: String,
    pub resultaten_bestand: String,
    pub api_key: Option<String>,
    pub toon_bieden: bool,
    pub toon_gratis: bool,
    pub toon_zie_beschrijving: bool,
    pub web_poort: u16,
    pub web_interface_aan: bool,
}


impl Default for Configuratie {
    fn default() -> Self {
        Configuratie {
            postcode: "3032SG".to_string(), // Dit is een verzonnen postcode 
            afstand_km: 8,
            check_interval_seconden: 300, 
            max_advertenties_per_zoekopdracht: 50,
            wenslijst_bestand: "wishlist.txt".to_string(),
            resultaten_bestand: "results.txt".to_string(),
            api_key: None,
            toon_bieden: true,
            toon_gratis: true,
            toon_zie_beschrijving: true,
            web_poort: 6600, // Willekeurig, ik heb niet gecheckt of dit conflict veroorzaakt
            web_interface_aan: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ZoekResultaten { 
    listings: Vec<Advertentie>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Advertentie {
    #[serde(rename = "itemId")]
    pub item_id: String,
    #[serde(rename = "title")]
    pub titel: String,
    #[serde(rename = "description")]
    pub beschrijving: Option<String>,
    #[serde(rename = "priceInfo")]
    pub prijs_info: PrijsInfo,
    #[serde(rename = "location")]
    pub locatie: Locatie,
    #[serde(rename = "vipUrl")]
    pub vip_url: String, // vip: View Item Page
    #[serde(rename = "imageUrls")]
    pub afbeelding_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct PrijsInfo {
    #[serde(rename = "priceCents")]
    pub prijs_centen: Option<i32>,
    #[serde(rename = "priceType")]
    pub prijs_type: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Locatie {
    #[serde(rename = "cityName")]
    pub stad_naam: Option<String>,
    #[serde(rename = "distanceMeters")]
    pub afstand_meters: Option<i32>,
}

#[derive(Debug)]
struct WenslijstItem {
    zoekwoord: String,
    max_prijs: i32,
}

pub struct Monitor {
    pub configuratie: Configuratie,
    pub gezien_advertenties: HashSet<String>,
}


// V Alle scraper functies zit hier V
impl Monitor { 
    pub fn nieuw(configuratie: Configuratie) -> Result<Self, Box<dyn Error>> {
        let mut monitor = Monitor {
            configuratie,
            gezien_advertenties: HashSet::new()
        };
        monitor.laad_bestaande_resultaten()?;
        Ok(monitor)
    }

    fn laad_bestaande_resultaten(&mut self) -> Result<(), Box<dyn Error>> {
        if Path::new(&self.configuratie.resultaten_bestand).exists() {
            let bestand = File::open(&self.configuratie.resultaten_bestand)?;
            let lezer = BufReader::new(bestand);

            for lijn in lezer.lines() {
                let lijn = lijn?;
                if lijn.contains("Link: ") {
                    if let Some(url) = lijn.split("Link: ").nth(1) {
                        self.gezien_advertenties.insert(url.trim().to_string()); // Juist formateren
                    }
                }
            }
            
            println!("Bestaande resultaten doorgenomen: {}", self.gezien_advertenties.len());
        }
        Ok(())
    }

    fn parseer_wenslijst(&self) -> Result<Vec<WenslijstItem>, Box<dyn Error>> {
        let mut wenslijst = Vec::new();
        
        if !Path::new(&self.configuratie.wenslijst_bestand).exists() {
            return Err(format!("{} niet gevonden!", self.configuratie.wenslijst_bestand).into()); 
        }

        let bestand = File::open(&self.configuratie.wenslijst_bestand)?;
        let lezer = BufReader::new(bestand);

        for (lijn_num, lijn) in lezer.lines().enumerate() {
            let lijn = lijn?; // controlle
            let lijn = lijn.trim();

            if lijn.is_empty() || lijn.starts_with("#") { 
                continue; // overslaan
            }

            if !lijn.contains(";") {
                eprintln!("Geen puntkomma (;) gevonden op lijn: {}, het wordt overgeslagen \nAls u geen maximum prijs wilt stellen voor een product kunt u -1 schrijven i.p.v {}", lijn_num + 1, lijn);
                continue;
            }

            let onderdelen: Vec<&str> = lijn.splitn(2, ";").collect();
            let zoekwoord = onderdelen[0].trim().to_string();

            let geparseerd = onderdelen[1].trim().parse::<i32>();
            let max_prijs = match geparseerd {
                Ok(n) if n > 0  => Some(n),
                Ok(-1)         => Some(i32::MAX), // Oneindig. Geen zin om een extra case toetevoegen in het zoeken
                Ok(0)          => Some(0),
                _              => None,
            };

            if let Some(prijs) = max_prijs {
                wenslijst.push(WenslijstItem { zoekwoord, max_prijs: prijs });
            } else {
                eprintln!("Probleem op lijn: {}, de prijs ({}) is ongeldig!", lijn_num + 1, lijn);
            }
            
        }

        println!("Artikelen doorgenomen van uw wensenlijst: {}", wenslijst.len());
        Ok(wenslijst)
    }

    fn advertentie_komt_overeen(&self, advertentie: &Advertentie, max_prijs: i32) -> bool {
        let prijs_type = advertentie.prijs_info.prijs_type.as_str();
        
        if let Some(centen) = advertentie.prijs_info.prijs_centen {
            if centen == 0 {
                if !self.configuratie.toon_gratis {
                    return false;
                }
                return true;
            }
            
            if max_prijs == 0 {
                return false;
            }
            
            if max_prijs == i32::MAX {
                return true;
            }
            
            return centen <= max_prijs * 100;
        }
        
        match prijs_type {
            "FREE" => {
                if !self.configuratie.toon_gratis {
                    return false;
                }
                return true;
            },
            "BID" => {
                if !self.configuratie.toon_bieden {
                    return false;
                }
                if max_prijs == 0 {
                    return false;
                }
                return true;
            },
            "SEE_DESCRIPTION" | "RESERVED" | "NOTK" | "MIN_BID" | "SWAP" => {
                if !self.configuratie.toon_zie_beschrijving {
                    return false;
                }
                if max_prijs == 0 {
                    return false;
                }
                return true;
            },
            _ => {
                if !self.configuratie.toon_zie_beschrijving {
                    return false;
                }
                if max_prijs == 0 {
                    return false;
                }
                return true;
            }
        }
    }

    pub async fn zoek_artikel(&self, zoekwoord: &str, max_prijs: i32) -> Result<Vec<Advertentie>, Box<dyn Error>> {
        let client = reqwest::Client::new();

        let prijs_centen = if max_prijs == i32::MAX { // dit was zulke hoofdpijn
            i64::MAX // Geen limiet
        } else {
            let prijs_i64 = max_prijs as i64;
            prijs_i64.saturating_mul(100) // Euros -> Centen
        };

        let url = format!(
            "https://www.marktplaats.nl/lrp/api/search?limit={}&offset=0&postcode={}&distanceMeters={}&priceFrom=0&priceTo={}&query={}",
            self.configuratie.max_advertenties_per_zoekopdracht,
            self.configuratie.postcode, 
            self.configuratie.afstand_km * 1000, // km -> m
            prijs_centen,
            urlencoding::encode(zoekwoord)
        );

        let user_agents = [
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            "Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/537.36",
            "Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/537.36",
            "Mozilla/5.0 (Windows NT 6.1; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_6) AppleWebKit/537.36",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",
            "Mozilla/5.0 (Linux; Android 13; SM-G991B) AppleWebKit/537.36",
            "Mozilla/5.0 (iPad; CPU OS 16_6 like Mac OS X) AppleWebKit/537.36",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 12_5_1) AppleWebKit/537.36",
            "Mozilla/5.0 (X11; Fedora; Linux x86_64) AppleWebKit/537.36",
            "Mozilla/5.0 (Linux; Android 12; OnePlus 9) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_14_6) AppleWebKit/537.36",
            "Mozilla/5.0 (Linux; Android 11; Nokia X20) AppleWebKit/537.36",
            "Mozilla/5.0 (Windows NT 6.3; Win64; x64) AppleWebKit/537.36",
            "Mozilla/5.0 (X11; CrOS x86_64 15604.45.0) AppleWebKit/537.36",
            "Mozilla/5.0 (Windows NT 10.0) AppleWebKit/537.36",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_13_6) AppleWebKit/537.36",
        ];

        let user_agent = user_agents[fastrand::usize(..user_agents.len())];
        let mut request_builder = client
            .get(&url)
            .header("User-Agent", user_agent);

        if let Some(api_key) = &self.configuratie.api_key {
            request_builder = request_builder.header("X-MP-Api-Key", api_key);
        }

        let antwoord = request_builder.send().await?;

        if !antwoord.status().is_success() {
            return Err(format!("HTTP probleem: {}", antwoord.status()).into())
        }

        let zoek_resultaten: ZoekResultaten = antwoord.json().await?;
        Ok(zoek_resultaten.listings)
    }
    pub fn formatteer_prijs(&self, advertentie: &Advertentie) -> String {
        match advertentie.prijs_info.prijs_centen {
            Some(centen) if centen == 0 => "Gratis".to_string(),
            Some(centen) => format!("€{:.2}", centen as f64 / 100.0),
            None => match advertentie.prijs_info.prijs_type.as_str() {
                "BID" => "Bieden".to_string(),
                "FREE" => "Gratis".to_string(),
                "SEE_DESCRIPTION" => "Zie beschrijving".to_string(),
                "RESERVED" => "Gereserveerd".to_string(),
                "NOTK" => "Nader overeen te komen".to_string(),
                "MIN_BID" => "Minimumbod".to_string(),
                "SWAP" => "Ruilen".to_string(),
                _ => "Zie beschrijving/anders".to_string(),
            }
        }
    }

    fn bewaar_resultaat(&mut self, zoekwoord: &str, max_prijs: i32, advertentie: &Advertentie) -> Result<(), Box<dyn Error>> {
        let tijdstempel = Local::now().format("%Y-%m-%d %H:%M:%S");
        let prijs_str = self.formatteer_prijs(advertentie);
        let locatie = advertentie.locatie.stad_naam.as_deref().unwrap_or("Onbekend");
        let volledige_url = format!("https://www.marktplaats.nl{}", advertentie.vip_url);

        let max_prijs_str = if max_prijs == i32::MAX {
            "onbeperkt".to_string()
        } else {
            max_prijs.to_string()
        };

        let afstand = match advertentie.locatie.afstand_meters {
            Some(meters) => format!("{:.1} km", meters as f64 / 1000.0),
            None => "Onbekend".to_string(),
        };

        let beschrijving = advertentie.beschrijving.as_ref().map(|d| {
            let afgekapt: String = d.chars().take(100).collect();
            if d.chars().count() > 100 {
                format!("{}...", afgekapt)
            } else {
                afgekapt
            }
        }).unwrap_or_else(|| "Geen beschrijving".to_string());

        let prijs_type_info = match advertentie.prijs_info.prijs_type.as_str() {
            "BID" => " [BIEDEN]",
            "FREE" => " [GRATIS]",
            "RESERVED" => " [GERESERVEERD]",
            "NOTK" => " [NOTK]",
            "MIN_BID" => " [MIN. BOD]",
            "SWAP" => " [RUILEN]",
            _ => "",
        };

        let afbeelding_url = advertentie.afbeelding_urls.as_ref()
            .and_then(|urls| urls.first())
            .map(|url| url.as_str())
            .unwrap_or("Geen afbeelding");

        let resultaat = format!(
            "[{}] Gevonden: \'{}\' (max €{})\n  Titel: {}\n  Prijs: {}{}\n  Locatie: {} ({})\n  Link: {}\n  Afbeelding: {}\n  Beschrijving: {}\n{}\n\n",
            tijdstempel, zoekwoord, max_prijs_str, advertentie.titel, prijs_str, prijs_type_info, locatie, afstand, volledige_url, afbeelding_url, beschrijving, "=".repeat(60)
        );

        let mut bestand = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.configuratie.resultaten_bestand)?;

        bestand.write_all(resultaat.as_bytes())?;

        println!("NIEUW: {} - {}{} - {}", advertentie.titel, prijs_str, prijs_type_info, volledige_url);

        Ok(())
    }

    async fn check_wenslijst(&mut self) -> Result<(), Box<dyn Error>> {
        let wenslijst = self.parseer_wenslijst()?;

        if wenslijst.is_empty() {
            eprintln!("Geen geldige artikelen gevonden in uw wensenlijst.");
            return Ok(());
        }

        let mut nieuw_aantal = 0;

        for item in wenslijst {
            let prijs_weergave = if item.max_prijs == i32::MAX {
                "onbeperkt".to_string()
            } else {
                format!("{}", item.max_prijs)
            };

            println!("Aan het zoeken voor: \'{}\' (max {} EUR)", item.zoekwoord, prijs_weergave);

            match self.zoek_artikel(&item.zoekwoord, item.max_prijs).await {
                Ok(advertenties) => {
                    for advertentie in advertenties {
                        let volledige_url = format!("https://www.marktplaats.nl{}", advertentie.vip_url);

                        let binnen_budget = self.advertentie_komt_overeen(&advertentie, item.max_prijs);

                        if !self.gezien_advertenties.contains(&volledige_url) && binnen_budget {
                            self.bewaar_resultaat(&item.zoekwoord, item.max_prijs, &advertentie)?;
                            self.gezien_advertenties.insert(volledige_url);
                            nieuw_aantal += 1;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Probleem met het zoeken voor \'{}\': {}", item.zoekwoord, e);
                }
            }

            thread::sleep(Duration::from_millis(500));
        }

        if nieuw_aantal > 0 {
            println!("Nieuwe resultaten: {}", nieuw_aantal);
        } else {
            println!("Niks nieuws gevonden.");
        }

        Ok(())
    }

    pub async fn draai(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Marktplaats Monitor");
        println!("Afstand: {}km van {} af", self.configuratie.afstand_km, self.configuratie.postcode);
        println!("Tussentijd: {} seconden(s)", self.configuratie.check_interval_seconden);
        println!("Resultaten Bestand: {}", self.configuratie.resultaten_bestand);
        println!("Wenslijst Bestand: {}\n", self.configuratie.wenslijst_bestand);

        if !Path::new(&self.configuratie.resultaten_bestand).exists() {
            let mut bestand = File::create(&self.configuratie.resultaten_bestand)?;
            let koptekst = format!(
                "Marktplaats Monitor Resultaten - Begonnen {}\nChecken {}km om {}\n{}\n\n",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                self.configuratie.afstand_km,
                self.configuratie.postcode,
                "=".repeat(64) // ziet er mooier uit
            );
            bestand.write_all(koptekst.as_bytes())?;
        }

        loop {
            println!("Wenslijst Checken [{}]", Local::now().format("%H:%M:%S"));
            if let Err(e) = self.check_wenslijst().await {
                eprintln!("Probleem aangekomen tijdens het checken: {}", e);
            }
            
            println!("Volgende check in {} seconden(s)...\n", self.configuratie.check_interval_seconden);
            thread::sleep(Duration::from_secs(self.configuratie.check_interval_seconden));
        }
    }
}

fn laad_of_maak_configuratie() -> Result<Configuratie, Box<dyn Error>> {
    let configuratie_bestand = "config.toml";
    
    if Path::new(configuratie_bestand).exists() {
        let inhoud = std::fs::read_to_string(configuratie_bestand)?;
        let configuratie: Configuratie = toml::from_str(&inhoud)?;
        println!("Config geladen van {}", configuratie_bestand);
        Ok(configuratie)
    } else {
        let configuratie = Configuratie::default();
        let toml_string = toml::to_string_pretty(&configuratie)?;
        std::fs::write(configuratie_bestand, toml_string)?;
        println!("Standaard config aangemaakt: {}", configuratie_bestand);
        Ok(configuratie)
    }
}

fn maak_voorbeeld_wenslijst(bestandsnaam: &str) -> Result<(), Box<dyn Error>> {
    let voorbeeld = r#"# Marktplaats Wensenlijst
# Formaat: zoekwoord;maximaleprijs
# Om te commenteren gebruikt u #
# Als u geen maximale prijs wilt, stelt u de prijs in als -1
# Wilt u gratis producten, doe 0 als de prijs

rx 6600;150
stoel;0
steam deck;-1
"#;
    std::fs::write(bestandsnaam, voorbeeld)?;
    println!("Voorbeeld wensenlijst aangemaaktt: {}", bestandsnaam);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let configuratie = laad_of_maak_configuratie()?;
    
    if !Path::new(&configuratie.wenslijst_bestand).exists() {
        maak_voorbeeld_wenslijst(&configuratie.wenslijst_bestand)?;
    }
    
    let mut monitor = Monitor::nieuw(configuratie.clone())?;
    
    if configuratie.web_interface_aan {
        let web_poort = configuratie.web_poort;
        let config_for_web = configuratie.clone();
        let web_config = Arc::new(std::sync::Mutex::new(config_for_web));
        
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                println!("Web interface wordt gestart op poort {}...", web_poort);
                web::start_web_server(web_poort, web_config).await;
            });
        });
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    monitor.draai().await
}