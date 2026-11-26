//! Discovery: the sweep that walks the portal and plans the work list.
//!
//! The sweep reads the cheap topology every run — the menu, the region
//! framesets, the regional and tournament menus, the championship indexes and
//! lists — and hashes each page against the ledger. A cached page the ledger
//! already holds with the same hash is not re-expanded, so an unchanged
//! portal yields an empty work list (docs/06-data-pipeline.md §3.1–§3.2). The
//! expansion levels below a championship and below a tournament index are
//! gated by that comparison: a new or changed championship expands down to
//! its matchday leaves, and the leaves — giornata pages, match details,
//! bracket homes, classification headers — become the fetch jobs the worker
//! drains. Discovery itself fetches only the pages it walks; the scheduler
//! records every walked page in the ledger.

mod ledger;
mod source;
mod surface;

pub use ledger::{Ledger, LedgerEntry, LedgerState};
pub use source::{ClientSource, Page, PageReply, PageSource};
pub use surface::Surface;

use std::time::{Duration, SystemTime};

use scraper::{Html, Selector};

use crate::archive::content_key;
use crate::conditional::CacheValidators;
use crate::error::FetchError;
use crate::html;
use crate::uri::{Uri, UriError};

/// The ledger name every portal row carries.
pub const SOURCE: &str = "fitet";

/// The menu page a source base URL seeds.
const MENU_PATH: &str = "menu.php";

/// The seed URL of one portal: its base URL plus the menu page.
pub fn seed(base_url: &str) -> Result<Uri, UriError> {
    let trimmed = base_url.trim_end_matches('/');
    Uri::parse(&format!("{trimmed}/{MENU_PATH}"))
}

/// The prototype's discovery cache window; the scheduler configures the real
/// TTL (docs/06-data-pipeline.md §3.1).
pub const DEFAULT_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Which attribute names a child page on its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkKind {
    Href,
    Frame,
}

impl LinkKind {
    fn selector(self) -> &'static str {
        match self {
            Self::Href => "a",
            Self::Frame => "frame",
        }
    }

    fn attribute(self) -> &'static str {
        match self {
            Self::Href => "href",
            Self::Frame => "src",
        }
    }
}

/// One walk edge: the parent surface, how the child is named, the child
/// surface, and whether the child is a fetch job rather than a walked page.
type Edge = (Surface, LinkKind, Surface, bool);

const EDGES: &[Edge] = &[
    (
        Surface::Menu,
        LinkKind::Href,
        Surface::ClassificationHeader,
        true,
    ),
    (
        Surface::Menu,
        LinkKind::Href,
        Surface::RegionFrameset,
        false,
    ),
    (
        Surface::Menu,
        LinkKind::Href,
        Surface::ChampionshipIndex,
        false,
    ),
    (
        Surface::Menu,
        LinkKind::Href,
        Surface::TournamentIndex,
        false,
    ),
    (
        Surface::RegionFrameset,
        LinkKind::Frame,
        Surface::RegionMenu,
        false,
    ),
    (
        Surface::RegionFrameset,
        LinkKind::Frame,
        Surface::TournamentMenu,
        false,
    ),
    (
        Surface::RegionMenu,
        LinkKind::Href,
        Surface::ChampionshipPage,
        false,
    ),
    (
        Surface::ChampionshipIndex,
        LinkKind::Frame,
        Surface::ChampionshipList,
        false,
    ),
    (
        Surface::ChampionshipList,
        LinkKind::Href,
        Surface::ChampionshipPage,
        false,
    ),
    (
        Surface::ChampionshipPage,
        LinkKind::Frame,
        Surface::ChampionshipHeader,
        false,
    ),
    (
        Surface::ChampionshipHeader,
        LinkKind::Href,
        Surface::Calendar,
        false,
    ),
    (
        Surface::Calendar,
        LinkKind::Href,
        Surface::GiornataHome,
        false,
    ),
    (
        Surface::GiornataHome,
        LinkKind::Frame,
        Surface::Giornata,
        true,
    ),
    (
        Surface::GiornataHome,
        LinkKind::Frame,
        Surface::MatchDetail,
        true,
    ),
    (
        Surface::TournamentMenu,
        LinkKind::Href,
        Surface::TournamentIndex,
        false,
    ),
    (
        Surface::TournamentIndex,
        LinkKind::Frame,
        Surface::TournamentHeader,
        false,
    ),
    (
        Surface::TournamentIndex,
        LinkKind::Frame,
        Surface::TournamentList,
        false,
    ),
    (
        Surface::TournamentList,
        LinkKind::Href,
        Surface::TournamentResults,
        false,
    ),
    (
        Surface::TournamentResults,
        LinkKind::Frame,
        Surface::BracketHome,
        true,
    ),
];

/// What one sweep produced.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Pages the sweep fetched and hashed; the scheduler records them.
    pub walked: Vec<Walked>,
    /// Fetch jobs for the worker, in deterministic order.
    pub jobs: Vec<Job>,
    /// URLs the source could not deliver; the branch was pruned.
    pub missing: Vec<Uri>,
}

/// One page the sweep fetched.
#[derive(Debug, Clone)]
pub struct Walked {
    pub surface: Surface,
    pub url: Uri,
    pub content_hash: String,
    pub validators: CacheValidators,
    /// True when the hash matched the ledger: the scheduler refreshes the
    /// row instead of rewriting it.
    pub unchanged: bool,
}

/// One fetch job for the worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub surface: Surface,
    pub url: Uri,
}

/// One discovery sweep over a page source and a ledger snapshot.
pub struct Discovery {
    seed: Uri,
    ttl: Duration,
}

impl Discovery {
    /// A sweep seeded at the portal menu page.
    pub fn new(seed: Uri) -> Self {
        Self {
            seed,
            ttl: DEFAULT_TTL,
        }
    }

    /// The TTL the ledger cache honors before a cached page is re-fetched.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Walk the portal and plan the work list. Deterministic: the plan sorts
    /// by surface, then by URL, whatever order the walk reached pages in.
    pub async fn plan(&self, walk: Walk<'_>) -> Result<Plan, DiscoveryError> {
        let mut walker = Walker {
            walk,
            ttl: self.ttl,
            plan: Plan {
                walked: Vec::new(),
                jobs: Vec::new(),
                missing: Vec::new(),
            },
        };
        walker.run(&self.seed).await?;
        Ok(walker.plan.sorted())
    }
}

/// Where one sweep reads its pages: the source, the ledger snapshot it diffs
/// against, and the clock the TTL is read against.
pub struct Walk<'a> {
    pub pages: &'a dyn PageSource,
    pub ledger: &'a Ledger,
    pub now: SystemTime,
}

/// Why a sweep failed: a fetch the facade refused, or a link the page spelled
/// in a form no resolver accepts.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("discovery could not fetch the portal: {0}")]
    Fetch(#[from] FetchError),

    #[error("a recorded link does not resolve: {0}")]
    Uri(#[from] UriError),
}

struct Walker<'a> {
    walk: Walk<'a>,
    ttl: Duration,
    plan: Plan,
}

impl Walker<'_> {
    async fn run(&mut self, seed: &Uri) -> Result<(), DiscoveryError> {
        let mut stack = vec![(Surface::Menu, seed.clone())];
        while let Some((surface, url)) = stack.pop() {
            if let Some(page) = self.visit(surface, &url).await? {
                let node = Node {
                    surface,
                    url: &url,
                    page: &page,
                };
                self.enqueue(&node, &mut stack)?;
            }
        }
        Ok(())
    }

    /// One page: a cached or missing one prunes the branch, a fresh one
    /// answers its parsed document.
    async fn visit(&mut self, surface: Surface, url: &Uri) -> Result<Option<Html>, DiscoveryError> {
        let entry = self.walk.ledger.get(url);
        let cached = entry.is_some_and(|entry| !entry.is_stale(self.walk.now, self.ttl));
        if cached && !surface.is_topology() {
            return Ok(None);
        }
        let validators = validators_for(entry, surface);
        match self.walk.pages.page(url, &validators).await? {
            PageReply::Missing => {
                self.plan.missing.push(url.clone());
                Ok(None)
            }
            PageReply::Unchanged => Ok(None),
            PageReply::Fresh(page) => self.record(Delivery { surface, url, page }),
        }
    }

    /// Hash a fresh page against the ledger and decide whether the sweep
    /// expands it.
    fn record(&mut self, delivery: Delivery<'_>) -> Result<Option<Html>, DiscoveryError> {
        let content_hash = content_key(&delivery.page.bytes);
        let unchanged = self
            .walk
            .ledger
            .get(delivery.url)
            .is_some_and(|entry| entry.content_hash == content_hash);
        self.plan.walked.push(Walked {
            surface: delivery.surface,
            url: delivery.url.clone(),
            content_hash,
            validators: delivery.page.validators,
            unchanged,
        });
        if unchanged && !delivery.surface.is_topology() {
            return Ok(None);
        }
        Ok(Some(html::parse_page(&delivery.page.bytes)))
    }

    /// Push the child pages of one walked page onto the stack, or into the
    /// job list when the child is a fetch target.
    fn enqueue(
        &mut self,
        node: &Node<'_>,
        stack: &mut Vec<(Surface, Uri)>,
    ) -> Result<(), DiscoveryError> {
        for (_, kind, child, job) in EDGES.iter().filter(|(from, ..)| *from == node.surface) {
            for target in children(node.page, node.url, (*child, *kind))? {
                if *job {
                    self.enqueue_job(*child, target);
                } else {
                    stack.push((*child, target));
                }
            }
        }
        Ok(())
    }

    /// A leaf the ledger already holds fresh needs no job: the worker
    /// re-checks it at its own cadence (docs/06-data-pipeline.md §4), and the
    /// discovery sweep stays cheap.
    fn enqueue_job(&mut self, surface: Surface, url: Uri) {
        let fresh = self
            .walk
            .ledger
            .get(&url)
            .is_some_and(|entry| !entry.is_stale(self.walk.now, self.ttl));
        if !fresh {
            self.plan.jobs.push(Job { surface, url });
        }
    }
}

/// One page the source delivered, with where it came from.
struct Delivery<'a> {
    surface: Surface,
    url: &'a Uri,
    page: Page,
}

/// One walked page its children are read from.
struct Node<'a> {
    surface: Surface,
    url: &'a Uri,
    page: &'a Html,
}

/// A fetch sends validators only for the cached expansion levels: the cheap
/// topology is re-read in full every sweep, because its links are the only
/// map the walk has.
fn validators_for(entry: Option<&LedgerEntry>, surface: Surface) -> CacheValidators {
    match (entry, surface.is_topology()) {
        (Some(entry), false) => entry.validators.clone(),
        _ => CacheValidators::default(),
    }
}

/// The child URLs of one walk edge: every link or frame whose resolved
/// target classifies as the edge's surface, sorted and deduplicated.
fn children(
    page: &Html,
    base: &Uri,
    edge: (Surface, LinkKind),
) -> Result<Vec<Uri>, DiscoveryError> {
    let selector = Selector::parse(edge.1.selector()).expect("link selector");
    let mut targets: Vec<Uri> = html::targets(page, &selector, edge.1.attribute())
        .iter()
        .map(|href| base.resolve(href))
        .collect::<Result<Vec<Uri>, UriError>>()?
        .into_iter()
        .filter(|uri| Surface::classify(uri) == Some(edge.0))
        .collect();
    targets.sort();
    targets.dedup();
    Ok(targets)
}

impl Plan {
    fn sorted(mut self) -> Self {
        self.walked
            .sort_by(|one, other| (one.surface, &one.url).cmp(&(other.surface, &other.url)));
        self.jobs
            .sort_by(|one, other| (one.surface, &one.url).cmp(&(other.surface, &other.url)));
        self.missing.sort();
        self.missing.dedup();
        self
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use async_trait::async_trait;

    use super::*;
    use crate::conditional::CacheValidators;

    /// The portal chain in miniature, page by page, with the URL shapes the
    /// recorded fixtures use: single-quoted hrefs, `..` segments, framesets.
    const MENU: &[u8] = br#"<html><body>
        <a href="r/default_reg.asp?REG=1">Valle d'Aosta</a>
        <a href="c/visualizza_campionati.php?CID=2">Under</a>
        <a href="s/testaclassifica_auto.php?ID_CLASS=248&ID=1&PASS=20">Classifica</a>
        <a href="t/risultati_tornei.php?TOR=9&COMIT=30&ID=0">Nazionali</a>
        <a href="scheda_atleti.php">Atleti</a>
    </body></html>"#;
    const FRAMESET: &[u8] = br#"<html><frameset>
        <frame src='menu_reg.php?REG=1'><frame src="menu_tor.php?REG=1">
        <frame src="testata.php?REG=1">
    </frameset></html>"#;
    const REGION_MENU: &[u8] = br#"<html><body><table width="120"></table></body></html>"#;
    const TOURNAMENT_MENU: &[u8] = br#"<html><body><table>
        <tr><td><a href="../tornei/risultati_tornei.php?TOR=110&COMIT=1&ID=0">Regionali</a></td></tr>
    </table></body></html>"#;
    const TOURNAMENT_FRAMESET: &[u8] = br#"<html><frameset>
        <frame src="testa_tornei.php?TOR=110&COMIT=1&ID=0">
        <frame src="elenco_tornei.php?TOR=110&COMIT=1&ID=0">
        <frame src="grigia_tor.php?TOR=110&COMIT=1">
    </frameset></html>"#;
    const TOURNAMENT_HEADER: &[u8] = br#"<html><body><table width="120"></table></body></html>"#;
    const TOURNAMENT_LIST: &[u8] = br#"<html><body>
        <a href="visualizza_risultati.php?IDT=7&ANNO=41&COM=1" target="inferiore">01/03/2026<br>Roma</a>
    </body></html>"#;
    const TOURNAMENT_RESULTS: &[u8] = br#"<html><frameset>
        <frame src="elenco_gare.php?IDT=7&COM=1">
        <frame src="grigia_logo.php?ANNO=41&IDT=7">
    </frameset></html>"#;
    const INDEX: &[u8] = br#"<html><frameset>
        <frame src="elenco_campionati.php?ID_CAMP=2">
    </frameset></html>"#;
    const LIST: &[u8] = br#"<html><body>
        <a href="campionati.php?CAM=1&CONC=0">Serie C</a>
        <a href="campionati.php?CAM=2&CONC=0">Serie D</a>
    </body></html>"#;
    const CHAMPIONSHIP: &[u8] = br#"<html><frameset>
        <frame src="testa_campionati.php?CAM=1&CONC=0">
        <frame src="classifica_squadre.php?CAM=1">
    </frameset></html>"#;
    const HEADER: &[u8] = br#"<html><body>
        <a href="Calendario.asp?CAM=1&ANNO=41">Calendario</a>
    </body></html>"#;
    const CALENDAR: &[u8] = br#"<html><body>
        <a href='giornata_home.php?INCONTRO=370793&CAM=1&FORMULA=17'>1</a>
        <a href='giornata_home.php?INCONTRO=370794&CAM=1&FORMULA=17'>2</a>
    </body></html>"#;
    const GIORNATA_HOME: &[u8] = br#"<html><frameset>
        <frame src="giornata.php?INCONTRO=370793&CAM=1&FORMULA=17">
        <frame src="dettaglio_partita.php?ID=1594362">
    </frameset></html>"#;

    /// A page source over recorded bytes; every other URL is missing, the way
    /// an unrecorded page prunes the fixture walk.
    struct MapSource(BTreeMap<String, &'static [u8]>);

    impl MapSource {
        fn portal() -> Self {
            let mut map = chain_tree();
            map.extend(tournament_tree());
            Self(map)
        }
    }

    /// The menu and the region, championship and matchday chain behind it.
    fn chain_tree() -> BTreeMap<String, &'static [u8]> {
        let base = "https://portale.fitet.org";
        let pages = [
            ("menu.php", MENU),
            ("r/default_reg.asp?REG=1", FRAMESET),
            ("r/menu_reg.php?REG=1", REGION_MENU),
            ("r/menu_tor.php?REG=1", TOURNAMENT_MENU),
            ("c/visualizza_campionati.php?CID=2", INDEX),
            ("c/elenco_campionati.php?ID_CAMP=2", LIST),
            ("c/campionati.php?CAM=1&CONC=0", CHAMPIONSHIP),
            ("c/testa_campionati.php?CAM=1&CONC=0", HEADER),
            ("c/Calendario.asp?ANNO=41&CAM=1", CALENDAR),
            (
                "c/giornata_home.php?CAM=1&FORMULA=17&INCONTRO=370793",
                GIORNATA_HOME,
            ),
        ];
        pages
            .map(|(path, bytes)| (format!("{base}/{path}"), bytes))
            .into()
    }

    /// The tournament branch of the miniature portal: the frameset, header and
    /// list the regional tournament index frames, and the results chain the
    /// list links.
    fn tournament_tree() -> BTreeMap<String, &'static [u8]> {
        let root = "https://portale.fitet.org/tornei";
        let pages = [
            (
                format!("{root}/risultati_tornei.php?COMIT=1&ID=0&TOR=110"),
                TOURNAMENT_FRAMESET,
            ),
            (
                format!("{root}/testa_tornei.php?COMIT=1&ID=0&TOR=110"),
                TOURNAMENT_HEADER,
            ),
            (
                format!("{root}/elenco_tornei.php?COMIT=1&ID=0&TOR=110"),
                TOURNAMENT_LIST,
            ),
            (
                format!("{root}/visualizza_risultati.php?ANNO=41&COM=1&IDT=7"),
                TOURNAMENT_RESULTS,
            ),
        ];
        pages.into_iter().collect()
    }

    #[async_trait]
    impl PageSource for MapSource {
        async fn page(
            &self,
            url: &Uri,
            _validators: &CacheValidators,
        ) -> Result<PageReply, FetchError> {
            Ok(match self.0.get(&url.as_url()) {
                Some(bytes) => PageReply::Fresh(Page {
                    bytes: bytes.to_vec(),
                    validators: CacheValidators::default(),
                }),
                None => PageReply::Missing,
            })
        }
    }

    fn seed() -> Uri {
        Uri::parse("https://portale.fitet.org/menu.php").expect("seed")
    }

    fn url(input: &str) -> Uri {
        Uri::parse(input).expect("url")
    }

    async fn sweep(ledger: &Ledger, ttl: Duration) -> Plan {
        let discovery = Discovery::new(seed()).with_ttl(ttl);
        discovery
            .plan(Walk {
                pages: &MapSource::portal(),
                ledger,
                now: SystemTime::now(),
            })
            .await
            .expect("plan")
    }

    #[tokio::test]
    async fn an_empty_ledger_plans_every_leaf_of_the_chain() {
        let plan = sweep(&Ledger::new(), DEFAULT_TTL).await;
        let jobs: Vec<String> = plan.jobs.iter().map(|job| job.url.as_url()).collect();
        assert_eq!(
            jobs,
            [
                "https://portale.fitet.org/s/testaclassifica_auto.php?ID=1&ID_CLASS=248&PASS=20",
                "https://portale.fitet.org/tornei/elenco_gare.php?COM=1&IDT=7",
                "https://portale.fitet.org/c/giornata.php?CAM=1&FORMULA=17&INCONTRO=370793",
                "https://portale.fitet.org/c/dettaglio_partita.php?ID=1594362",
            ]
        );
    }

    /// Every page the miniature portal walk reads, in the surface order the
    /// plan sorts into.
    const WALKED_CHAIN: &[Surface] = &[
        Surface::Menu,
        Surface::RegionFrameset,
        Surface::RegionMenu,
        Surface::TournamentMenu,
        Surface::TournamentIndex,
        Surface::TournamentHeader,
        Surface::TournamentList,
        Surface::TournamentResults,
        Surface::ChampionshipIndex,
        Surface::ChampionshipList,
        Surface::ChampionshipPage,
        Surface::ChampionshipHeader,
        Surface::Calendar,
        Surface::GiornataHome,
    ];

    #[tokio::test]
    async fn the_sweep_walks_every_page_of_the_chain() {
        let plan = sweep(&Ledger::new(), DEFAULT_TTL).await;
        let walked: Vec<Surface> = plan.walked.iter().map(|page| page.surface).collect();
        assert_eq!(walked, WALKED_CHAIN);
    }

    #[tokio::test]
    async fn unrecorded_links_are_reported_missing() {
        let plan = sweep(&Ledger::new(), DEFAULT_TTL).await;
        assert_eq!(
            plan.missing,
            [
                url("https://portale.fitet.org/c/campionati.php?CAM=2&CONC=0"),
                url(
                    "https://portale.fitet.org/c/giornata_home.php?CAM=1&FORMULA=17&INCONTRO=370794"
                ),
                url("https://portale.fitet.org/t/risultati_tornei.php?COMIT=30&ID=0&TOR=9"),
            ]
        );
    }

    #[tokio::test]
    async fn an_unchanged_portal_plans_no_jobs() {
        let first = sweep(&Ledger::new(), DEFAULT_TTL).await;
        let ledger = ledger_of(&first, SystemTime::now());
        let second = sweep(&ledger, DEFAULT_TTL).await;
        assert!(second.jobs.is_empty(), "{:?}", second.jobs);
        // why: the pruned championship and tournament branches no longer
        // re-read their links, so only the unrecorded championship and the
        // unrecorded national tournament index the menu links stay missing
        assert_eq!(second.missing.len(), 2);
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
    async fn an_expired_cache_re_reads_the_unchanged_championship() {
        let first = sweep(&Ledger::new(), DEFAULT_TTL).await;
        let stale = SystemTime::now() - DEFAULT_TTL - Duration::from_secs(60);
        let second = sweep(&ledger_of(&first, stale), DEFAULT_TTL).await;
        // why: the stale expansion pages are re-read, an unchanged one plans no
        // new work, and only the stale classification header stays a job
        assert!(
            second
                .walked
                .iter()
                .any(|page| page.surface == Surface::ChampionshipPage)
        );
        assert!(second.walked.iter().all(|page| page.unchanged));
        assert_eq!(second.jobs.len(), 1);
    }

    /// The ledger the scheduler would have written after one sweep: one row per
    /// walked page, and one row per leaf job the worker then fetched. why: the
    /// worker holds the real hashes of the leaves; the discovery gate only asks
    /// whether the entry is fresh.
    fn ledger_of(plan: &Plan, fetched_at: SystemTime) -> Ledger {
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

    fn row(
        content_hash: String,
        validators: CacheValidators,
        fetched_at: SystemTime,
    ) -> LedgerEntry {
        LedgerEntry {
            content_hash,
            validators,
            state: LedgerState::Fetched,
            fetched_at,
        }
    }
}
