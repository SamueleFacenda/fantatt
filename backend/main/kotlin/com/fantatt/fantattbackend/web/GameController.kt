package com.fantatt.fantattbackend.web

import com.fantatt.fantattbackend.db.entities.League
import com.fantatt.fantattbackend.db.entities.Participation
import com.fantatt.fantattbackend.db.entities.Society
import com.fantatt.fantattbackend.game.LeagueCreator
import com.fantatt.fantattbackend.db.repos.LeagueRepository
import com.fantatt.fantattbackend.db.repos.SocietyRepository
import com.fantatt.fantattbackend.game.LineupManager
import org.springframework.web.bind.annotation.*
import java.security.Principal

@RestController
class GameController(
    val leagueCreator: LeagueCreator,
    val leagueRepository: LeagueRepository,
    val societyRepository: SocietyRepository,
    val lineupManager: LineupManager
) {

    @PostMapping("/league/create")
    @ResponseBody
    fun createLeague(@RequestParam name: String, @RequestBody teams: List<Society>, principal: Principal): League {
        return leagueCreator.leagueFrom(
            masterName = principal.name,
            leagueName = name,
            teams = teams
        )
    }

    @GetMapping("/league/{teamId}/scoreboard")
    @ResponseBody
    fun getScoreboard(@PathVariable teamId: Long) = societyRepository.findAllByLeagueIdOrderByPointsDesc(teamId)

    @GetMapping("/team/{teamId}/lineup")
    @ResponseBody
    fun getLineup(@PathVariable teamId: Long): List<Participation> {
        val team = societyRepository.findById(teamId).orElseThrow { Exception("Team not found") }
        return lineupManager.getLineup(team)
    }

}