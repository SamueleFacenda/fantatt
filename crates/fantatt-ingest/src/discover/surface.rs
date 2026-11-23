//! The portal surfaces discovery walks, mirroring the fixture surface kinds
//! (G1.1). why: fantatt-fixtures is a dev-only crate, so the production walk
//! carries its own vocabulary.

use crate::uri::Uri;

/// One source surface of the results portal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Surface {
    Menu,
    RegionFrameset,
    RegionMenu,
    TournamentMenu,
    ChampionshipIndex,
    ChampionshipList,
    ChampionshipPage,
    ChampionshipHeader,
    Calendar,
    GiornataHome,
    Giornata,
    MatchDetail,
    TournamentIndex,
}

impl Surface {
    /// Classify a URL by the page it names. None for surfaces discovery does
    /// not walk: the regional home and header, the classification pages, the
    /// athlete sheets, and the documents the menu also links.
    pub fn classify(uri: &Uri) -> Option<Self> {
        let page = uri.path().rsplit('/').next().unwrap_or_default();
        match page {
            "menu.php" => Some(Self::Menu),
            "default_reg.asp" => Some(Self::RegionFrameset),
            "menu_reg.php" => Some(Self::RegionMenu),
            "menu_tor.php" => Some(Self::TournamentMenu),
            "visualizza_campionati.php" => Some(Self::ChampionshipIndex),
            "elenco_campionati.php" => Some(Self::ChampionshipList),
            "campionati.php" => Some(Self::ChampionshipPage),
            "testa_campionati.php" => Some(Self::ChampionshipHeader),
            "Calendario.asp" => Some(Self::Calendar),
            "giornata_home.php" => Some(Self::GiornataHome),
            "giornata.php" => Some(Self::Giornata),
            "dettaglio_partita.php" => Some(Self::MatchDetail),
            "risultati_tornei.php" => Some(Self::TournamentIndex),
            _ => None,
        }
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
            Self::RegionFrameset => "region-frameset",
            Self::RegionMenu => "region-menu",
            Self::TournamentMenu => "tournament-menu",
            Self::ChampionshipIndex => "championship-index",
            Self::ChampionshipList => "championship-list",
            Self::ChampionshipPage => "championship-page",
            Self::ChampionshipHeader => "championship-header",
            Self::Calendar => "calendar",
            Self::GiornataHome => "giornata-home",
            Self::Giornata => "giornata",
            Self::MatchDetail => "match-detail",
            Self::TournamentIndex => "tournament-index",
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
            (
                "risultati/tornei/risultati_tornei.php?TOR=110",
                Surface::TournamentIndex,
            ),
        ] {
            let input = format!("https://portale.fitet.org/{input}");
            assert_eq!(Surface::classify(&uri(&input)), Some(expected), "{input}");
        }
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
            "https://portale.fitet.org/risultati/new_rank/testaclassifica_auto.php",
            "https://portale.fitet.org/risultati/scheda_atleti.php",
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
