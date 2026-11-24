//! Golden discovery: the sweep over the recorded portal tree (G1.1), asserting
//! the work list of docs/06-data-pipeline.md §3.1 and the idempotence of §3.2.
//! Every page comes from tests/fixtures/fitet; no test reaches the network.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use fantatt_fixtures::FixtureSet;
use fantatt_ingest::conditional::CacheValidators;
use fantatt_ingest::discover::{
    DEFAULT_TTL, Discovery, Ledger, LedgerEntry, LedgerState, Page, PageReply, PageSource, Surface,
    Walk,
};
use fantatt_ingest::{FetchError, uri::Uri};

/// The recorded tree as a page source: every URL the capture index names, and
/// nothing else, so the walk stops at the recorded boundary the way a portal
/// without the page would prune the branch.
struct RecordedTree(BTreeMap<String, Vec<u8>>);

impl RecordedTree {
    fn load() -> Self {
        let set = FixtureSet::discover().expect("fixture tree");
        let mut pages = BTreeMap::new();
        for surface in set.html_surfaces().expect("surfaces") {
            let bytes = std::fs::read(set.file_path(&surface.file)).expect("page");
            let key = Uri::parse(&surface.url).expect("recorded url");
            pages.insert(key.as_url(), bytes);
        }
        Self(pages)
    }
}

#[async_trait]
impl PageSource for RecordedTree {
    async fn page(
        &self,
        url: &Uri,
        _validators: &CacheValidators,
    ) -> Result<PageReply, FetchError> {
        Ok(match self.0.get(&url.as_url()) {
            Some(bytes) => PageReply::Fresh(Page {
                bytes: bytes.clone(),
                validators: CacheValidators::default(),
            }),
            None => PageReply::Missing,
        })
    }
}

fn seed() -> Uri {
    Uri::parse("https://portale.fitet.org/menu.php").expect("seed")
}

async fn sweep(
    tree: &RecordedTree,
    ledger: &Ledger,
    ttl: Duration,
) -> fantatt_ingest::discover::Plan {
    Discovery::new(seed())
        .with_ttl(ttl)
        .plan(Walk {
            pages: tree,
            ledger,
            now: SystemTime::now(),
        })
        .await
        .expect("plan")
}

/// The ledger the scheduler writes after a sweep: one row per walked page and
/// one row per leaf job the worker then fetched. why: the worker holds the
/// real hashes of the leaves; the gate only asks whether the entry is fresh.
fn ledger_of(plan: &fantatt_ingest::discover::Plan, fetched_at: SystemTime) -> Ledger {
    let mut ledger = Ledger::new();
    for page in &plan.walked {
        let hash = page.content_hash.clone();
        ledger.insert(&page.url, row(hash, page.validators.clone(), fetched_at));
    }
    for job in &plan.jobs {
        let entry = row(String::new(), CacheValidators::default(), fetched_at);
        ledger.insert(&job.url, entry);
    }
    ledger
}

fn row(content_hash: String, validators: CacheValidators, fetched_at: SystemTime) -> LedgerEntry {
    LedgerEntry {
        content_hash,
        validators,
        state: LedgerState::Fetched,
        fetched_at,
    }
}

#[tokio::test]
async fn the_recorded_tree_plans_the_expected_work_list() {
    let tree = RecordedTree::load();
    let plan = sweep(&tree, &Ledger::new(), DEFAULT_TTL).await;
    let jobs = &plan.jobs;

    // the menu links 16 national tournaments, the regional tournament menu
    // links 9 more, and the one recorded giornata home frames its referto and
    // its match detail
    assert_eq!(jobs.len(), 27, "{jobs:?}");
    let count = |surface| jobs.iter().filter(|job| job.surface == surface).count();
    assert_eq!(count(Surface::TournamentIndex), 25);
    assert_eq!(count(Surface::Giornata), 1);
    assert_eq!(count(Surface::MatchDetail), 1);
}

#[tokio::test]
async fn the_sweep_walks_the_whole_recorded_chain() {
    let tree = RecordedTree::load();
    let plan = sweep(&tree, &Ledger::new(), DEFAULT_TTL).await;
    let walked: Vec<Surface> = plan.walked.iter().map(|page| page.surface).collect();
    assert_eq!(
        walked,
        [
            Surface::Menu,
            Surface::RegionFrameset,
            Surface::RegionMenu,
            Surface::TournamentMenu,
            Surface::ChampionshipIndex,
            Surface::ChampionshipList,
            Surface::ChampionshipPage,
            Surface::ChampionshipHeader,
            Surface::Calendar,
            Surface::GiornataHome,
        ]
    );
}

#[tokio::test]
async fn unrecorded_links_are_reported_and_commented_out_ones_are_not() {
    let tree = RecordedTree::load();
    let plan = sweep(&tree, &Ledger::new(), DEFAULT_TTL).await;

    // the recorded boundary: 20 further regions, 7 further indexes (the
    // commented-out ones never reach the walk), 7 further championships, and
    // 55 further matchdays
    assert_eq!(plan.missing.len(), 89);
    assert!(
        plan.missing
            .iter()
            .all(|url| !url.as_url().contains("CID=5")),
        "a commented-out link is not a fetch target"
    );
}

#[tokio::test]
async fn the_leaves_carry_their_identifiers() {
    let tree = RecordedTree::load();
    let plan = sweep(&tree, &Ledger::new(), DEFAULT_TTL).await;
    let urls: Vec<String> = plan.jobs.iter().map(|job| job.url.as_url()).collect();
    // the OPEN tournament drops the ID parameter the other types carry
    for expected in [
        "https://portale.fitet.org/risultati/campionati/giornata.php?CAM=1&FORMULA=17&INCONTRO=370793",
        "https://portale.fitet.org/risultati/campionati/dettaglio_partita.php?ID=1594362",
        "https://portale.fitet.org/risultati/tornei/risultati_tornei.php?COMIT=1&ID=0&TOR=110",
        "https://portale.fitet.org/risultati/tornei/risultati_tornei.php?COMIT=30&ID=0&TOR=9",
        "https://portale.fitet.org/risultati/tornei/risultati_tornei.php?COMIT=1&TOR=100",
    ] {
        assert!(urls.iter().any(|url| url == expected), "{expected}");
    }
}

#[tokio::test]
async fn re_running_over_unchanged_fixtures_plans_no_jobs() {
    let tree = RecordedTree::load();
    let first = sweep(&tree, &Ledger::new(), DEFAULT_TTL).await;
    let ledger = ledger_of(&first, SystemTime::now());
    let second = sweep(&tree, &ledger, DEFAULT_TTL).await;
    assert!(second.jobs.is_empty(), "{:?}", second.jobs);

    // the cheap topology is still re-read; the expansion levels stay cached
    let rewalked: Vec<Surface> = second.walked.iter().map(|page| page.surface).collect();
    assert_eq!(
        rewalked,
        [
            Surface::Menu,
            Surface::RegionFrameset,
            Surface::RegionMenu,
            Surface::TournamentMenu,
            Surface::ChampionshipIndex,
            Surface::ChampionshipList,
        ]
    );
    assert!(second.walked.iter().all(|page| page.unchanged));
}

#[tokio::test]
async fn the_plan_is_deterministic() {
    let tree = RecordedTree::load();
    let one = sweep(&tree, &Ledger::new(), DEFAULT_TTL).await;
    let two = sweep(&tree, &Ledger::new(), DEFAULT_TTL).await;
    assert_eq!(jobs_of(&one), jobs_of(&two));
    assert_eq!(walked_of(&one), walked_of(&two));
}

fn jobs_of(plan: &fantatt_ingest::discover::Plan) -> Vec<String> {
    plan.jobs.iter().map(|job| job.url.as_url()).collect()
}

fn walked_of(plan: &fantatt_ingest::discover::Plan) -> Vec<String> {
    plan.walked.iter().map(|page| page.url.as_url()).collect()
}
