package com.fantatt.fantattbackend.web

import com.fantatt.fantattbackend.db.entities.League
import com.fantatt.fantattbackend.db.entities.Team
import com.fantatt.fantattbackend.game.LeagueCreator
import com.fantatt.fantattbackend.db.repos.LeagueRepository
import com.fantatt.fantattbackend.db.repos.TeamRepository
import org.springframework.web.bind.annotation.*
import java.security.Principal

@RestController
class GameController(
    val leagueCreator: LeagueCreator,
    val leagueRepository: LeagueRepository,
    val teamRepository: TeamRepository
) {

    @PostMapping("/league/create")
    @ResponseBody
    fun createLeague(@RequestParam name: String, @RequestBody teams: List<Team>, principal: Principal): League {
        return leagueCreator.leagueFrom(
            masterName = principal.name,
            leagueName = name,
            teams = teams
        )
    }

    @GetMapping("/league/{id}/scoreboard")
    @ResponseBody
    fun getScoreboard(@PathVariable id: Long) = teamRepository.findAllByLeagueIdOrderByPointsDesc(id)

    @GetMapping("/team/{id}/lineup")
    @ResponseBody
    fun getLineup(@PathVariable id: Long) {
        TODO()
    }

}