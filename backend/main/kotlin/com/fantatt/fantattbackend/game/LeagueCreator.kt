package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.League
import com.fantatt.fantattbackend.db.entities.Team
import com.fantatt.fantattbackend.db.repos.LeagueRepository
import com.fantatt.fantattbackend.db.repos.TeamRepository
import com.fantatt.fantattbackend.db.repos.UserRepository
import org.springframework.stereotype.Component

@Component
class LeagueCreator(
    private val userRepository: UserRepository,
    private val leagueRepository: LeagueRepository,
    private val seasonManager: SeasonManager,
    private val teamRepository: TeamRepository
) {
    fun leagueFrom(masterName: String, leagueName: String, teams: List<Team>): League {
        checkName(leagueName)
        checkTeamList(teams)
        val master = userRepository.findByUsername(masterName)?: throw Exception("User $masterName not found")
        val league = leagueRepository.save(League(
            name = leagueName,
            master = master,
            teams = teams.toMutableList(),
            season = seasonManager.getCurrent()
        ))
        teamRepository.saveAll(teams.map { it.copy(league = league) })
        return league
    }

    private fun checkName(leagueName: String) {
        if (leagueName.isBlank())
            throw Exception("League name cannot be blank")
    }

    private fun checkTeamList(teams: List<Team>) {
        if (teams.size < 2)
            throw Exception("League must have at least 2 teams")

        if (teams.size > 10)
            throw Exception("League cannot have more than 10 teams")

        checkOneUserPerTeam(teams)
        checkPlayersPerTeam(teams)
        checkNames(teams)
    }

    private fun checkOneUserPerTeam(teams: List<Team>) {
        checkAllDifferent(teams.map { it.owner }, "Each team must have a different user")
    }

    private fun checkPlayersPerTeam(teams: List<Team>) {
        checkAllDifferent(teams.flatMap { it.players }, "Each team must have different players")
    }

    private fun checkNames(teams: List<Team>) {
        checkAllDifferent(teams.map { it.name }, "Each team must have a different name")
    }

    private fun <T> checkAllDifferent(objs: List<T>, msg: String) {
        if (objs.size != objs.distinct().size)
            throw Exception(msg)
    }
}