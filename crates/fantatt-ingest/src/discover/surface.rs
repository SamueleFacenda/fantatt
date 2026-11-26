//! The portal surfaces discovery walks, mirroring the fixture surface kinds
//! (G1.1). why: fantatt-fixtures is a dev-only crate, so the production walk
//! carries its own vocabulary.

use crate::uri::Uri;

/// One page file of the portal and the surface it names.
struct PageName {
    file: &'static str,
    surface: Surface,
}

/// Every walked page file, in walk order. why: a table keeps the vocabulary
/// and the classification in one place, so a new surface lands once.
const PAGES: &[PageName] = &[
    PageName {
        file: "menu.php",
        surface: Surface::Menu,
    },
    PageName {
        file: "testaclassifica_auto.php",
        surface: Surface::ClassificationHeader,
    },
    PageName {
        file: "default_reg.asp",
        surface: Surface::RegionFrameset,
    },
    PageName {
        file: "menu_reg.php",
        surface: Surface::RegionMenu,
    },
    PageName {
        file: "menu_tor.php",
        surface: Surface::TournamentMenu,
    },
    PageName {
        file: "risultati_tornei.php",
        surface: Surface::TournamentIndex,
    },
    PageName {
        file: "testa_tornei.php",
        surface: Surface::TournamentHeader,
    },
    PageName {
        file: "elenco_tornei.php",
        surface: Surface::TournamentList,
    },
    PageName {
        file: "visualizza_risultati.php",
        surface: Surface::TournamentResults,
    },
    PageName {
        file: "elenco_gare.php",
        surface: Surface::BracketHome,
    },
    PageName {
        file: "visualizza_campionati.php",
        surface: Surface::ChampionshipIndex,
    },
    PageName {
        file: "elenco_campionati.php",
        surface: Surface::ChampionshipList,
    },
    PageName {
        file: "campionati.php",
        surface: Surface::ChampionshipPage,
    },
    PageName {
        file: "testa_campionati.php",
        surface: Surface::ChampionshipHeader,
    },
    PageName {
        file: "Calendario.asp",
        surface: Surface::Calendar,
    },
    PageName {
        file: "giornata_home.php",
        surface: Surface::GiornataHome,
    },
    PageName {
        file: "giornata.php",
        surface: Surface::Giornata,
    },
    PageName {
        file: "dettaglio_partita.php",
        surface: Surface::MatchDetail,
    },
];

/// One source surface of the results portal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Surface {
    Menu,
    ClassificationHeader,
    RegionFrameset,
    RegionMenu,
    TournamentMenu,
    TournamentIndex,
    TournamentHeader,
    TournamentList,
    TournamentResults,
    BracketHome,
    ChampionshipIndex,
    ChampionshipList,
    ChampionshipPage,
    ChampionshipHeader,
    Calendar,
    GiornataHome,
    Giornata,
    MatchDetail,
}

impl Surface {
    /// Classify a URL by the page it names. None for surfaces discovery does
    /// not walk: the regional home and header, the ranking and athlete pages,
    /// and the documents the menu also links. why: the ranking rows and the
    /// athlete sheets answer no link a page spells; their coordinates come
    /// from the classification header, so they stay parser targets only.
    pub fn classify(uri: &Uri) -> Option<Self> {
        let page = uri.path().rsplit('/').next().unwrap_or_default();
        PAGES
            .iter()
            .find(|entry| entry.file == page)
            .map(|entry| entry.surface)
    }

    /// True for the cheap topology a sweep re-fetches every run and always
    /// reads for links; false for the expansion levels the ledger cache
    /// gates (docs/06-data-pipeline.md §3.1).
    pub fn is_topology(self) -> bool {
        matches!(
            self,
            Self::Menu
                | Self::RegionFrameset
                | Self::RegionMenu
                | Self::TournamentMenu
                | Self::ChampionshipIndex
                | Self::ChampionshipList
        )
    }

    /// The stable name a job of this surface carries.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::ClassificationHeader => "classification-header",
            Self::RegionFrameset => "region-frameset",
            Self::RegionMenu => "region-menu",
            Self::TournamentMenu => "tournament-menu",
            Self::TournamentIndex => "tournament-index",
            Self::TournamentHeader => "tournament-header",
            Self::TournamentList => "tournament-list",
            Self::TournamentResults => "tournament-results",
            Self::BracketHome => "bracket-home",
            Self::ChampionshipIndex => "championship-index",
            Self::ChampionshipList => "championship-list",
            Self::ChampionshipPage => "championship-page",
            Self::ChampionshipHeader => "championship-header",
            Self::Calendar => "calendar",
            Self::GiornataHome => "giornata-home",
            Self::Giornata => "giornata",
            Self::MatchDetail => "match-detail",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(input: &str) -> Uri {
        Uri::parse(input).expect("url")
    }

    #[test]
    fn the_region_surfaces_classify_by_their_page() {
        for (input, expected) in [
            ("menu.php", Surface::Menu),
            (
                "risultati/regioni/default_reg.asp?REG=1",
                Surface::RegionFrameset,
            ),
            ("risultati/regioni/menu_reg.php?REG=2", Surface::RegionMenu),
            (
                "risultati/regioni/menu_tor.php?REG=1",
                Surface::TournamentMenu,
            ),
        ] {
            let input = format!("https://portale.fitet.org/{input}");
            assert_eq!(Surface::classify(&uri(&input)), Some(expected), "{input}");
        }
    }

    #[test]
    fn the_tournament_index_and_header_classify_by_their_page() {
        for (input, expected) in [
            (
                "risultati/tornei/risultati_tornei.php?TOR=110",
                Surface::TournamentIndex,
            ),
            (
                "risultati/tornei/testa_tornei.php?TOR=100&COMIT=1&ID=0",
                Surface::TournamentHeader,
            ),
            (
                "risultati/tornei/elenco_tornei.php?TOR=100&COMIT=1&ID=0",
                Surface::TournamentList,
            ),
        ] {
            let input = format!("https://portale.fitet.org/{input}");
            assert_eq!(Surface::classify(&uri(&input)), Some(expected), "{input}");
        }
    }

    #[test]
    fn the_tournament_results_surfaces_classify_by_their_page() {
        for (input, expected) in [
            (
                "risultati/tornei/visualizza_risultati.php?IDT=1&ANNO=41&COM=6",
                Surface::TournamentResults,
            ),
            (
                "risultati/tornei/elenco_gare.php?IDT=1&COM=6",
                Surface::BracketHome,
            ),
        ] {
            let input = format!("https://portale.fitet.org/{input}");
            assert_eq!(Surface::classify(&uri(&input)), Some(expected), "{input}");
        }
    }

    #[test]
    fn the_classification_header_classifies_by_its_page() {
        let input = "https://portale.fitet.org/risultati/new_rank/testaclassifica_auto.php?ID_CLASS=248&ID=1&PASS=20";
        assert_eq!(
            Surface::classify(&uri(input)),
            Some(Surface::ClassificationHeader)
        );
    }

    #[test]
    fn the_championship_list_surfaces_classify_by_their_page() {
        for (input, expected) in [
            (
                "c/visualizza_campionati.php?CID=2",
                Surface::ChampionshipIndex,
            ),
            (
                "c/elenco_campionati.php?ID_CAMP=2",
                Surface::ChampionshipList,
            ),
            ("c/campionati.php?CAM=1&CONC=0", Surface::ChampionshipPage),
        ] {
            let input = format!("https://portale.fitet.org/{input}");
            assert_eq!(Surface::classify(&uri(&input)), Some(expected), "{input}");
        }
    }

    #[test]
    fn the_matchday_surfaces_classify_by_their_page() {
        for (input, expected) in [
            (
                "c/testa_campionati.php?CAM=1&CONC=0",
                Surface::ChampionshipHeader,
            ),
            ("c/Calendario.asp?CAM=1&ANNO=41", Surface::Calendar),
            ("c/giornata_home.php?INCONTRO=370793", Surface::GiornataHome),
            ("c/giornata.php?INCONTRO=370793", Surface::Giornata),
            ("c/dettaglio_partita.php?ID=1594362", Surface::MatchDetail),
        ] {
            let input = format!("https://portale.fitet.org/{input}");
            assert_eq!(Surface::classify(&uri(&input)), Some(expected), "{input}");
        }
    }

    #[test]
    fn a_surface_discovery_does_not_walk_stays_unclassified() {
        for input in [
            "https://portale.fitet.org/risultati/new_rank/ranking_unica.php?SESSO=M&ID_CLASS=248",
            "https://portale.fitet.org/risultati/new_rank/dettaglioatleta_unica.php?ATLETA=737436",
            "https://portale.fitet.org/risultati/new_rank/ajax.php?term=kozul",
            "https://portale.fitet.org/risultati/scheda_atleti.php",
            "https://portale.fitet.org/risultati/tornei/grigia_tor.php?TOR=100&COMIT=1",
            "https://portale.fitet.org/risultati/campionati/classifica_squadre.php?CAM=1",
            "http://archivio.fitet.org/",
        ] {
            assert_eq!(Surface::classify(&uri(input)), None, "{input}");
        }
    }

    #[test]
    fn the_topology_surfaces_are_the_cheap_ones() {
        assert!(Surface::Menu.is_topology());
        assert!(Surface::ChampionshipList.is_topology());
        assert!(!Surface::ChampionshipPage.is_topology());
        assert!(!Surface::GiornataHome.is_topology());
        assert!(!Surface::Giornata.is_topology());
    }
}
