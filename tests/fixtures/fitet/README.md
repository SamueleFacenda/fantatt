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
└── html/                   # current-portal HTML surfaces, captured 2026-11-23
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
    ├── dettaglio_partita_1594362.html     # dettaglio_partita.php?ID=1594362: PUPPO–KOZUL
    └── ajax_kozul.json                    # ajax.php?term=kozul: the player search reply
```

The chain is deliberately complete: `menu.php` frames point at the region and
championship surfaces, the region frameset carries the regional championship
menu and the tournament type menu, the calendar lists the `INCONTRO` link, the
giornata home frames the referto, and `ajax.php?term=kozul` resolves the same
athlete id (`737436`) that `dettaglio_partita.php` names — that cross-reference
is an assertion in `fantatt-fixtures`.

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