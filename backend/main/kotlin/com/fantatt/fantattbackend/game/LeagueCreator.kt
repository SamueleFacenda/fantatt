package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.League
import com.fantatt.fantattbackend.db.entities.Society
import com.fantatt.fantattbackend.db.entities.Team
import com.fantatt.fantattbackend.db.repos.LeagueRepository
import com.fantatt.fantattbackend.db.repos.SocietyRepository
import com.fantatt.fantattbackend.db.repos.TeamRepository
import com.fantatt.fantattbackend.db.repos.UserRepository
import org.springframework.stereotype.Component

@Component
class LeagueCreator(
    private val userRepository: UserRepository,
    private val leagueRepository: LeagueRepository,
    private val calendarManager: CalendarManager,
    private val societyRepository: SocietyRepository,
    private val teamRepository: TeamRepository
) {
    fun leagueFrom(masterName: String, leagueName: String, societies: List<Society>, nDivisions: Int = 3): League {
        checkName(leagueName)
        checkTeamList(societies)
        val master = userRepository.findByUsername(masterName)?: throw Exception("User $masterName not found")
        val league = leagueRepository.save(League(
            name = leagueName,
            master = master,
            societies = societies.toMutableList(),
            season = calendarManager.getCurrentSeason()
        ))
        val updatedSocieties = societies.map { it.copy(league = league) }
        societyRepository.saveAll(updatedSocieties)
        societies.forEach { generateSocietyTeams(it, nDivisions) }
        return league
    }

    private fun checkName(leagueName: String) {
        if (leagueName.isBlank())
            throw Exception("League name cannot be blank")
    }

    private fun checkTeamList(teams: List<Society>) {
        if (teams.size < 2)
            throw Exception("League must have at least 2 teams")

        if (teams.size > 10)
            throw Exception("League cannot have more than 10 teams")

        checkOneUserPerTeam(teams)
        checkPlayersPerTeam(teams)
        checkNames(teams)
    }

    private fun checkOneUserPerTeam(teams: List<Society>) {
        checkAllDifferent(teams.map { it.owner }, "Each team must have a different user")
    }

    private fun checkPlayersPerTeam(teams: List<Society>) {
        checkAllDifferent(teams.flatMap { it.players }, "Each team must have different players")
    }

    private fun checkNames(teams: List<Society>) {
        checkAllDifferent(teams.map { it.name }, "Each team must have a different name")
    }

    private fun <T> checkAllDifferent(objs: List<T>, msg: String) {
        if (objs.size != objs.distinct().size)
            throw Exception(msg)
    }

    private fun generateSocietyTeams(society: Society, nDivisions: Int) {
        teamRepository.saveAll(
            generateDefaultNames(nDivisions).mapIndexed { index, name ->
                Team(
                    society = society,
                    division = index + 1,
                    name = name
                )
            }
        )
    }

    private fun generateDefaultNames(len: Int): List<String> {
        // should be enough for every league, crashes if there are more
        return ('A'..'Z').take(len).map { it.toString() }
    }
}
