# Marktplaats Monitor
Een Marktplaats monitor die draait in uw browser.

<img width="1917" height="756" alt="image" src="https://github.com/user-attachments/assets/cb286209-6de6-4530-85d4-3e6e9fb84301" />

Zijn functie is Marktplaats monitoren, doormiddel van zijn best uitvoerig doordachte [API](https://api.marktplaats.nl/docs/v1/index.html)
Eens u het programma start wordt er een server gestart op **poort 6600**. U kunt erin uw wenslijst aanpassen, de resultaten doorzoeken, en de configuratie aanpassen. U kunt ook de .txt & .toml bestanden die gemaakt worden naast het programma aanpassen.

Als u vaardig bent met python scripts schrijven raad ik [marktplaats-py door JensjeFlensje](https://github.com/jensjeflensje/marktplaats-py) aan.. [Hier is een voorbeeld van een Marktplaats Monitor die ik schreef voor termux met marktplaats-py.](https://gist.github.com/Servus-Altissimi/a765f2041e7c3b0cdf643a3055ca20f7) Echter geloof ik zelf dat mijn implementatie niet snel overtroffen wordt door een script geschreven met marktplaats-py.

Toch vindt ik het bestaan van dit project terecht. Ik hoop dat door te werken met Rust & Markplaats' API i.p.v. marktplaats-py het mogelijk wordt de monitor efficienter te laten draaien, en functies toevoegen die buiten Marktplaats' wensen valt. Dat moet echter nog volbracht worden.

## Compileren
```
cargo build --release
```

