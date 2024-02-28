package com.fantatt.fantattbackend.web

import com.fantatt.fantattbackend.db.entities.League
import com.fantatt.fantattbackend.db.entities.Participation
import com.fantatt.fantattbackend.db.entities.Player
import com.fantatt.fantattbackend.db.entities.Society
import com.fantatt.fantattbackend.db.repos.*
import com.fantatt.fantattbackend.game.CalendarManager
import com.fantatt.fantattbackend.game.LeagueCreator
import com.fantatt.fantattbackend.game.LineupManager
import org.springframework.web.bind.annotation.*
import java.security.Principal

@RestController
class GameController(
    val leagueCreator: LeagueCreator,
    val leagueRepository: LeagueRepository,
    val societyRepository: SocietyRepository,
    val lineupManager: LineupManager,
    val teamRepository: TeamRepository,
    val userRepository: UserRepository,
    val roundRepository: RoundRepository,
    val calendarManager: CalendarManager
) {

    @PostMapping("/league/create")
    @ResponseBody
    fun createLeague(@RequestParam name: String, @RequestBody teams: List<Society>, principal: Principal): League {
        val master = userRepository.findByUsername(principal.name) ?: throw Exception("User not found")
        return leagueCreator.buildFrom(
            master = master,
            leagueName = name,
            societies = teams
        )
    }

    @GetMapping("/league/{leagueId}/scoreboard")
    @ResponseBody
    fun getScoreboard(@PathVariable leagueId: Long) = societyRepository.findAllByLeagueIdOrderByPointsDesc(leagueId)

    @GetMapping("/team/{societyId}/{teamName}/lineup")
    @ResponseBody
    fun getLineup(@PathVariable societyId: Long, @PathVariable teamName: String, @RequestParam roundIndex: Int): List<Participation> {
        val team = teamRepository.findBySocietyIdAndName(societyId, teamName) ?: throw Exception("Team not found")
        val round = roundRepository.findByIndexAndSeason(roundIndex, calendarManager.getCurrentSeason()) ?: throw Exception("Round not found")
        return lineupManager.getLineup(team, round)
    }

    @GetMapping("/society/{societyId}/players")
    @ResponseBody
    fun getPlayers(@PathVariable societyId: Long): List<Player> {
        val society = societyRepository.findById(societyId).orElseThrow { Exception("Society not found") }
        return society.players
    }

    // set team lineup


    //


}