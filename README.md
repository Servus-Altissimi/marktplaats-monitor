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
