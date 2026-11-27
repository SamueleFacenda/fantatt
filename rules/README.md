# Rule bundles

Versioned, content-addressed game logic under `<name>/<semver>/`.

```
rules/
├── fantatt-core/1.0.0/
│   ├── manifest.toml                  # name, version, entry points, requires, evidence
│   ├── ontology/*.ttl                 # vocabulary the entry points read and assert into
│   ├── rules/*.n3                     # game logic, one file per entry point
│   ├── queries/*.rq                   # SPARQL reports over the derived facts
│   └── tests/golden/<entrypoint>/
│       ├── input.ttl                  # the graph the entry point reads
│       ├── expected.ttl               # the facts it must derive
│       └── expected.proof             # the certificate it must emit
└── ...
```

A bundle is an immutable artifact: `fantatt-rules` loads one only when its
manifest agrees with its directory, every declared entry point names both a rule
file and a golden case, and no file the layout does not declare rides along.
Loading then hashes the manifest and every file the bundle carries — sorted by
relative path, each contribution length-prefixed — into one `rulesetVersion`, so
an edited rule reads as a new version rather than a silent change.

`just rules` loads every committed bundle and reports the version each one
addresses. The engine that evaluates the rules and emits the proof certificates
lands with the reasoning-engine goal (docs/04-reasoning-rules.md §3 and §9).