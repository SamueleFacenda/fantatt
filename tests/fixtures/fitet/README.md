# FITET golden fixtures

Recorded fixtures for the ingest pipeline (goal G1.1). Every test that parses,
asserts, or reconciles FITET data reads this tree through the
`fantatt-fixtures` crate; no test reaches the network.

## Layout

```
tests/fixtures/fitet/
├── matches.json            # the legacy match corpus, recovered verbatim from
│                           # the prototype (git commit 444d843, assets/matches.json):
│                           # 1085 records, {one, two, score: [[points, points]],
│                           # source: {type, a, b, date}}; 522 championship
│                           # giornate (partita) and 563 tournament matches (torneo),
│                           # played 2023-10-07 to 2023-11-21
├── capture.json            # the capture index: kind, file, and source URL of
│                           # every recorded surface (fixtures are located by
│                           # kind, not by hardcoded paths)
├── manifest.sha256         # one sha256 line per file (README excluded)
└── html/                   # current-portal HTML surfaces; the base set was
                            # captured 2026-11-23 and the giornata set the
                            # G1.4 parser builds against on 2026-11-25
    ├── menu.html                          # menu.php: the frameset entry point
    ├── default_reg_1.html                 # default_reg.asp?REG=1: the Valle d'Aosta frameset
    ├── menu_reg_1.html                    # menu_reg.php?REG=1: the regional championship menu
    ├── menu_reg_2.html                    # menu_reg.php?REG=2: a menu with championships
    ├── menu_tor_1.html                    # menu_tor.php?REG=1: the tournament type menu
    ├── bianca_reg_1.html                  # bianca.php?REG=1: the regional calendar shell
    ├── testata_reg_1.html                 # testata.php?REG=1: the regional committee header
    ├── visualizza_campionati_2.html       # visualizza_campionati.php?CID=2
    ├── elenco_campionati_2.html           # elenco_campionati.php?ID_CAMP=2
    ├── campionati_cam1.html               # campionati.php?CAM=1&CONC=0
    ├── testa_campionati_cam1.html         # testa_campionati.php?CAM=1&CONC=0
    ├── Calendario_cam1_a41.html           # Calendario.asp?CAM=1&ANNO=41 (season 2026/27)
    ├── giornata_home_370793.html          # giornata_home.php?INCONTRO=370793&CAM=1&FORMULA=17
    ├── giornata_370793.html               # giornata.php: the Sassari–Muravera referto
    ├── giornata_370794.html               # giornata.php: Marcozzi–Reggio Emilia, five ties
    ├── giornata_370796.html               # giornata.php: Apuania–Sassari, four ties
    ├── giornata_370800.html               # giornata.php: Muravera–Apuania, three ties
    ├── giornata_370805.html               # giornata.php: Santa Tecla–Siracusa, five ties
    ├── giornata_370808.html               # giornata.php: five ties, three decided 3-2
    ├── giornata_392016.html               # giornata.php: scheduled, no tie played yet
    ├── giornata_392017.html              # giornata.php: scheduled, date not confirmed
    ├── giornata_376199.html               # giornata.php: CAM=201, seven ties
    ├── giornata_376201.html               # giornata.php: CAM=201, a 1-6 matchday
    ├── giornata_376203.html               # giornata.php: CAM=201, includes a doubles tie
    ├── giornata_376255.html               # giornata.php: CAM=202, seven ties
    ├── dettaglio_partita_1594362.html     # dettaglio_partita.php?ID=1594362: PUPPO–KOZUL
    ├── ajax_kozul.json                    # ajax.php?term=kozul: the player search reply
    ├── classifica_header_248.html         # testaclassifica_auto.php?ID_CLASS=248: the classification header
    ├── ranking_unica_248_m_p1.html        # ranking_unica.php: the general male ranking, first page
    ├── ranking_unica_248_f_p1.html        # ranking_unica.php: the general female ranking, first page
    ├── ranking_unica_245_m_p1.html        # ranking_unica.php: a second classification's first page
    ├── dettaglioatleta_737436.html        # dettaglioatleta_unica.php: the searched athlete's sheet
    ├── dettaglioatleta_710705.html        # dettaglioatleta_unica.php: the top-ranked athlete's sheet
    ├── risultati_tornei_100_1.html        # risultati_tornei.php?TOR=100: the tournament frameset
    ├── testa_tornei_100_1.html            # testa_tornei.php: the tournament type header
    ├── elenco_tornei_100_1.html           # elenco_tornei.php: a committee with no tournament listed
    ├── elenco_tornei_100_6.html           # elenco_tornei.php: one tournament listed
    ├── elenco_tornei_100_7.html           # elenco_tornei.php: two tournaments listed
    ├── elenco_tornei_100_9.html           # elenco_tornei.php: six tournaments listed
    ├── visualizza_risultati_1_41.html     # visualizza_risultati.php: the tournament results frameset
    ├── elenco_gare_1_6.html               # elenco_gare.php: the bracket home of the Trento tournament
    ├── visualizza_risultati_2_41.html     # visualizza_risultati.php: a second tournament's frameset
    └── elenco_gare_2_9.html               # elenco_gare.php: a second tournament's bracket home
```

The chain is deliberately complete: `menu.php` frames point at the region and
championship surfaces, the region frameset carries the regional championship
menu and the tournament type menu, the calendar lists the `INCONTRO` link, the
giornata home frames the referto, and `ajax.php?term=kozul` resolves the same
athlete id (`737436`) that `dettaglio_partita.php` names — that cross-reference
is an assertion in `fantatt-fixtures`.

The classification axis is recorded alongside it: the classification header
page drives its ranking tables through XHR calls whose parameters the page
spells in its `onclick` handlers, and the ranking rows name the athlete ids the
player sheets carry. The tournament axis keeps both states of a committee
listing: `elenco_tornei.php?TOR=100&COMIT=1` answers with an empty table, the
committees above it list tournaments whose results frame the bracket home
`elenco_gare.php` renders. The non-empty committee listings are constructed
URLs rather than links the frameset names, so discovery reaches them by
sweeping the committee range of a tournament type instead of by following a
page.

`menu_reg.php?REG=1` is recorded with an empty table on purpose: Valle d'Aosta
runs no regional championships this season, so discovery must treat a menu
without links as a complete answer rather than a broken page. `menu_reg.php?REG=2`
carries the populated counterpart (41 championships through `../campionati/`
relative paths).

The portal grammar changed since the prototype (2023): framesets moved from
`menu.php?REG=` paths to `?CID=`/`?CAM=`/`?ANNO=` query identifiers, and
`giornata_home.php` wraps the referto that `giornata.php` rendered. Parser
goals G1.4–G1.5 build against these pages; the 2023 corpus is the expected-output
side of the golden set, and the current pages are the input side.

## Regenerating

`just fixtures-capture` re-records the `html/` tree from the live portal and
rewrites `capture.json`. Add a surface by adding a line to the table in
`scripts/fixtures-capture`; never hand-edit captured bytes.

`just fixtures-verify` (part of `just check`) walks the tree, verifies every
file against `manifest.sha256`, fails on an unlisted or missing file, parses
every declared match array, and prints the record and surface counts.