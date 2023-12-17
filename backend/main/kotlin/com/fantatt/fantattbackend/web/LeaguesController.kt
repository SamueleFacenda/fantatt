package com.fantatt.fantattbackend.web

import com.fantatt.fantattbackend.db.entities.League
import com.fantatt.fantattbackend.db.entities.Team
import com.fantatt.fantattbackend.db.repos.LeagueRepository
import com.fantatt.fantattbackend.db.repos.TeamRepository
import com.fantatt.fantattbackend.db.repos.UserRepository
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestParam
import org.springframework.web.bind.annotation.RequestBody
import org.springframework.web.bind.annotation.RestController
import java.security.Principal

@RestController
class LeaguesController(
    teamRepository: TeamRepository,
    userRepository: UserRepository,
    leagueRepository: LeagueRepository
) {

    @PostMapping("/league/create")
    fun createLeague(@RequestParam name: String, @RequestBody teams: List<Team>, principal: Principal) {
        if (name.isBlank())
            throw Exception("Name cannot be blank")

        checkTeamList(teams)

        val master = userRepository.findByUsername(principal.name)
        val league = League(name, master, teams)
    }

    fun checkTeamList(teams: List<Team>) {
        if (teams.size < 2)
            throw Exception("League must have at least 2 teams")

        if (teams.size > 10)
            throw Exception("League cannot have more than 10 teams")

        checkOneUserPerTeam(teams)
        checkPlayersPerTeam(teams)
    }

    fun checkOneUserPerTeam(teams: List<Team>) {
        val users = teams.map { it.owner }
        if (users.size != users.distinct().size)
            throw Exception("Each team must have a different user")
    }

    fun checkPlayersPerTeam(teams: List<Team>) {
        val players = teams.flatMap { it.players }
        if (players.size != players.distinct().size)
            throw Exception("Each team must have different players")
    }
}