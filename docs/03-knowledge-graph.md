# Knowledge Graph Model

The knowledge graph (KG) is the single representation of game state. It is not a
general-purpose database: it holds *facts and rules*, and the reasoning engine
answers questions about them. Transactional records that need CRUD semantics
(accounts, invitations, audit) stay in PostgreSQL; the KG holds the game world.

## 1. Design rules

1. **Named, versioned ontology.** One namespace, one version, one release.
2. **No blank nodes in game facts.** Everything is addressable by IRI so a score
   can be cited in a dispute.
3. **Provenance is mandatory.** Every asserted triple carries source, actor,
   timestamp, and rule/import version.
4. **Closed-world within a snapshot for validation; open-world for discovery.**
   Roster validity is checked closed-world (missing fact = invalid). Player
   suggestions are checked open-world (absence is not proof).
5. **Bitemporal facts.** Each fact has `assertedAt` and `validFrom`. Ratings
   change; a matchday computed with an old rating must be replayable.

## 2. Ontology namespaces

```
@prefix tt:   <https://fantatt.org/ontology/tt#>          # core vocabulary
@prefix rule: <https://fantatt.org/rules/>                # indexed by ruleset version
@prefix data: <https://fantatt.org/data/>                 # instance data
@prefix prov: <http://www.w3.org/ns/prov#>                # PROV-O
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#>
@prefix owl:  <http://www.w3.org/2002/07/owl#>
@prefix sh:   <http://www.w3.org/ns/shacl#>               # validation shapes
```

## 3. Vocabulary (turtle extract)

```turtle
tt:Player a owl:Class ;
  rdfs:label "Player" ;
  sh:property [ sh:path tt:playerId ; sh:datatype xsd:string ; sh:minCount 1 ] ,
              [ sh:path tt:fullName ; sh:datatype xsd:string ; sh:minCount 1 ] ,
              [ sh:path tt:gender ; sh:in ( tt:Male tt:Female ) ] .

tt:fullName a owl:DatatypeProperty ; rdfs:domain tt:Player ; rdfs:range xsd:string .
tt:gender   a owl:DatatypeProperty ; rdfs:domain tt:Player ; rdfs:range tt:Gender .
tt:rating   a owl:DatatypeProperty ; rdfs:domain tt:Player ; rdfs:range xsd:integer .
tt:ratingAsOf a owl:DatatypeProperty ; rdfs:range xsd:date .

tt:DivisionTier a owl:Class ;
  sh:property [ sh:path tt:tierOrder ; sh:datatype xsd:integer ] .
tt:B2 tt:tierOrder 1 . tt:C1 tt:tierOrder 2 . tt:C2 tt:tierOrder 3 .
tt:D1 tt:tierOrder 4 . tt:D2 tt:tierOrder 5 .

tt:Competition a owl:Class ;
  sh:property [ sh:path tt:divisionTier ; sh:class tt:DivisionTier ] ,
              [ sh:path tt:region ; sh:datatype xsd:string ] ,
              [ sh:path tt:seasonYear ; sh:datatype xsd:integer ] .

tt:Matchday a owl:Class ;
  sh:property [ sh:path tt:number ; sh:datatype xsd:integer ] ,
              [ sh:path tt:scheduledDate ; sh:datatype xsd:date ] ,
              [ sh:path tt:status ; sh:in ( tt:Scheduled tt:Partial tt:Complete tt:Cancelled ) ] .

tt:Tie a owl:Class ;
  sh:property [ sh:path tt:format ; sh:class tt:TeamFormat ] ,
              [ sh:path tt:hasAppearance ; sh:class tt:PlayerAppearance ] .

tt:PlayerAppearance a owl:Class ;
  sh:property [ sh:path tt:matchday ; sh:class tt:Matchday ] ,
              [ sh:path tt:appearanceKind ; sh:in ( tt:Singles tt:Doubles ) ] ,
              [ sh:path tt:played ; sh:datatype xsd:boolean ] ,
              [ sh:path tt:outcome ; sh:in ( tt:Win tt:Loss ) ] ,
              [ sh:path tt:setsWon ; sh:datatype xsd:integer ] ,
              [ sh:path tt:setsLost ; sh:datatype xsd:integer ] ,
              [ sh:path tt:isAway ; sh:datatype xsd:boolean ] ,
              [ sh:path tt:isDecider ; sh:datatype xsd:boolean ] ,
              [ sh:path tt:isComeback ; sh:datatype xsd:boolean ] .

tt:League a owl:Class ;
  sh:property [ sh:path tt:ownershipMode ; sh:in ( tt:Exclusive tt:Shared ) ] ,
              [ sh:path tt:startingBudget ; sh:datatype xsd:decimal ] ,
              [ sh:path tt:rulesetVersion ; sh:datatype xsd:string ] ,
              [ sh:path tt:drawsFrom ; sh:class tt:Competition ] .

tt:Squad a owl:Class ;
  sh:property [ sh:path tt:month ; sh:datatype xsd:gYearMonth ] ,
              [ sh:path tt:hasSlot ; sh:class tt:SquadSlot ] ,
              [ sh:path tt:status ; sh:in ( tt:Draft tt:Submitted tt:Locked tt:Scored ) ] .

tt:SquadSlot a owl:Class ;
  sh:property [ sh:path tt:slotTier ; sh:class tt:DivisionTier ] ,
              [ sh:path tt:slotKind ; sh:in ( tt:Starter tt:Bench ) ] ,
              [ sh:path tt:slotPlayer ; sh:class tt:Player ] .
```

## 4. Instance data example

```turtle
data:player/8f2a a tt:Player ;
  tt:playerId "8f2a..." ;
  tt:fullName "Facenda Samuele" ;
  tt:gender tt:Male ;
  tt:rating 4070 ; tt:ratingAsOf "2026-10-01" ;
  tt:playsIn data:competition/c2-trentino-a ;
  tt:memberOf data:team/trentino-alfa .

data:matchday/2026-11-08/c2-trentino-a/1 a tt:Matchday ;
  tt:number 1 ; tt:scheduledDate "2026-11-08" ; tt:status tt:Complete ;
  tt:inCompetition data:competition/c2-trentino-a ;
  tt:hasTie data:tie/2026-11-08/alfa-beta .

data:tie/2026-11-08/alfa-beta a tt:Tie ;
  tt:format tt:Swaythling ;
  tt:homeTeam data:team/trentino-alfa ;
  tt:awayTeam data:team/trentino-beta ;
  tt:homeScore 5 ; tt:awayScore 3 ;
  tt:winner data:team/trentino-alfa .

data:appearance/8f2a/2026-11-08/1 a tt:PlayerAppearance ;
  tt:player data:player/8f2a ;
  tt:matchday data:matchday/2026-11-08/c2-trentino-a/1 ;
  tt:tie data:tie/2026-11-08/alfa-beta ;
  tt:appearanceKind tt:Singles ;
  tt:played true ;
  tt:outcome tt:Win ;
  tt:setsWon 3 ; tt:setsLost 2 ;
  tt:isAway true ;
  tt:isDecider false ;
  tt:isComeback false ;
  tt:isTeamWin true ;
  prov:wasAttributedTo data:import/2026-11-08-crawl ;
  tt:bitemporalValidFrom "2026-11-08"^^xsd:date ;
  tt:bitemporalAssertedAt "2026-11-08T20:41:03Z"^^xsd:dateTime .
```

Doubles:

```turtle
data:appearance/8f2a/2026-11-08/dbl a tt:PlayerAppearance ;
  tt:appearanceKind tt:Doubles ;
  tt:player data:player/8f2a ;
  tt:partner data:partner-iri ;
  tt:outcome tt:Win ;
  tt:setsWon 3 ; tt:setsLost 1 .
```

## 5. Derived facts (materialised by the rule engine)

```turtle
data:squad/s1/month/2026-11 a tt:Squad ;
  tt:status tt:Locked ;
  tt:hasSlot [ a tt:SquadSlot ; tt:slotTier tt:D1 ; tt:slotKind tt:Starter ; tt:slotPlayer data:player/8f2a ] .

data:score/squad-s1/matchday-1 a tt:SquadMatchdayScore ;
  tt:value 33.224 ;
  tt:derivedFrom data:slot/s1/d1/0 ;
  tt:ruleBundle <https://fantatt.org/rules/scoring/1.4.0> ;
  tt:computedFromRevision "a19f..." .
```

## 6. Query patterns

### 6.1 Which players are eligible for a D1 starter slot?

```sparql
SELECT ?player ?rating WHERE {
  ?player a tt:Player ;
          tt:rating ?rating ;
          tt:gender tt:Male ;
          tt:playsIn ?comp .
  ?comp tt:divisionTier tt:D1 .
  FILTER NOT EXISTS { ?squad a tt:Squad ; tt:hasSlot ?s .
                      ?s tt:slotPlayer ?player ; tt:slotTier tt:D1 . }
}
ORDER BY DESC(?rating)
```

### 6.2 Which matchdays are complete?

```sparql
SELECT ?md (COUNT(?tie) AS ?total) (SUM(IF(?tie tt:winner ?w, 1, 0)) AS ?decided)
WHERE {
  ?md a tt:Matchday ; tt:hasTie ?tie .
  OPTIONAL { ?tie tt:winner ?w }
} GROUP BY ?md HAVING (COUNT(?tie) > SUM(IF(?tie tt:winner ?w, 1, 0)))
```

### 6.3 Player cost inputs (for the cost index)

```sparql
SELECT ?player ?rating
       (COUNT(DISTINCT ?app) AS ?matches)
       (AVG(?oppRatingAdj) AS ?avgOpponentStrength)
WHERE {
  ?app a tt:PlayerAppearance ; tt:player ?player ; tt:played true ;
       tt:opponentRatingAdjusted ?oppRatingAdj .
  ?player tt:rating ?rating .
}
GROUP BY ?player ?rating
```

## 7. Storage strategy

| Requirement | Approach |
| --- | --- |
| Embeddable in the API process | Store the triples in a native RDF dataset held by the binary. |
| Serverless / multi-node | Snapshot to object storage; nodes hydrate on boot; a single writer owns ingest. |
| Big data volume | Keep only the current season hot; archive prior seasons as compressed triple dumps, queryable on demand. |
| Full verification | Any past matchday can be recomputed from the archived triples + the rule bundle version recorded on the score. |

The graph is a *derived-cache-with-provenance* model: PostgreSQL holds the
authoritative ingest ledger, the KG holds the queryable game semantics. If the
KG is lost, it is rebuilt from the ledger plus the rule bundles. That keeps the
KG cheap to run and disposable, which is what makes it cost effective.
