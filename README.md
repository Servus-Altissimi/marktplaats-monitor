# Marktplaats Monitor
Een relatief simpele Marktplaats monitor voor in de terminal.

## Compileren
```
cargo build --release
```
Ik heb dit enkel getest op mijn FreeBSD machine, ik weet niet of dit werkt op andere POSIX systemen. 

## Functies
De functie is Marktplaats monitoren, doormiddel van zijn best uitvoerig doordachte [API](https://api.marktplaats.nl/docs/v1/index.html)  Op dit moment kan het enkel een lijst aflezen van gegeven gewenste producten, met daarbij een maximale prijskaartje en afstand van uw postcode. Het is eenmaal op dit moment enorm eenvoudig, maar ik denk dat ik wel wat aanpassingen ga doen terloops. 

Ik zou het zelf, persoonlijk, nog niet gebruiken, [marktplaats-py door JensjeFlensje](https://github.com/jensjeflensje/marktplaats-py) is een veel betere toepassing van Marktplaats' API. [Hier is een voorbeeld van een Marktplaats Monitor die ik schreef voor termux met marktplaats-py.](https://gist.github.com/Servus-Altissimi/a765f2041e7c3b0cdf643a3055ca20f7)

Toch vindt ik het bestaan van dit project terecht. Ik hoop dat door te werken met Rust & Markplaats' API i.p.v. marktplaats-py het mogelijk wordt de monitor efficienter te laten draaien, en functies toevoegen die buiten Marktplaats' wensen valt. Dat moet echter nog volbracht worden.

## Te-Doen
- Notificaties
- Beter omgaan met biedingen en gratis producten
- Resultaten opsplitsen
- Android versie
- Automatisch bieden
- Automatisch berichten verzenden


        body { font-family: Arial; margin: 20px; background: #f5f5f5; min-height: 100vh; display: flex; flex-direction: column; }
        .content { flex: 1; }
        h1 { color: #333; }
        .zoekbalk { margin: 20px 0; }
        input[type="text"] { padding: 8px; width: 300px; }
        button { padding: 8px 16px; background:rgb(255, 143, 68); color: white; border: none; cursor: pointer; }
        button:hover { background: #0056b3; }
        .resultaat { background: white; padding: 15px; margin: 10px 0; border: 1px solid #ddd; border-radius: 5px; }
        .resultaat img { max-width: 150px; max-height: 150px; float: left; margin-right: 15px; border-radius: 5px; object-fit: cover; }
        .resultaat h3 { margin: 0 0 10px 0; }
        .resultaat a { color: #007bff; text-decoration: none; }
        .resultaat a:hover { text-decoration: underline; }
        .prijs { font-weight: bold; color:rgb(0, 190, 44); }
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
        footer { margin-top: 40px; padding: 20px; text-align: center; background: #333; color: white; border-radius: 5px; }
        footer a { color: #4db8ff; text-decoration: none; }
        footer a:hover { text-decoration: underline; }


  <p>Een product van<a href="https://constringo.com" target="_blank">Constringo</a>, neem contact!</p>        <p style="font-size: 12px; margin-top: 10px;">Marktplaats Monitor © 2025</p>
