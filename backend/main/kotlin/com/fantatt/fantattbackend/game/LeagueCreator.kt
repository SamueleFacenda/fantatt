package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.*
import com.fantatt.fantattbackend.db.repos.*
import org.springframework.stereotype.Component
import kotlin.random.Random

@Component
class LeagueCreator(
    private val userRepository: UserRepository,
    private val leagueRepository: LeagueRepository,
    private val calendarManager: CalendarManager,
    private val societyRepository: SocietyRepository,
    private val teamRepository: TeamRepository,
    private val matchRepository: MatchRepository
) {
    fun buildFrom(master: User, leagueName: String, societies: List<Society>, nDivisions: Int = 3): League {
        checkName(leagueName)
        checkTeamList(societies)
        val league = leagueRepository.save(League(
            name = leagueName,
            master = master,
            societies = societies.toMutableList(),
            season = calendarManager.getCurrentSeason()
        ))
        val updatedSocieties = societies.map { it.copy(league = league) }
        societyRepository.saveAll(updatedSocieties)
        societies.forEach { generateSocietyTeams(it, nDivisions) }
        generateMatches(league)
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

    private fun generateMatches(league: League, doReturnRound: Boolean = false) {
        val rounds = calendarManager.getRemainingRounds()
        // https://en.wikipedia.org/wiki/Round-robin_tournament?useskin=vector#Scheduling_algorithm
        var matches = generateRoundRobin(league.societies.size)
        if (doReturnRound)
            matches += matches.map { it.map { pair -> pair.second to pair.first } }

        val teams = league.societies.flatMap { it.teams }.groupBy { it.division }
        teams.map { (division, teams) -> generateDivision(teams, matches, rounds) }
            .forEach { matchRepository.saveAll(it) }
    }

    private fun generateRoundRobin(n: Int): List<List<Pair<Int, Int>>> {
        // Richard Schurig algorithm (prefer clarity to performance)
        val nMatchesPerRound: Int = n / 2
        val nRounds = n - 1
        val firstPlayer = List(nMatchesPerRound * nRounds) { it % n }
            .chunked(nMatchesPerRound)
        val secondPlayer = firstPlayer.drop(1) + firstPlayer.take(1)
        val matches = firstPlayer.zip(secondPlayer) { first, second -> first.zip(second) }
        return if (n % 2 == 0) {
            // add the n-th player to the first match of every round
            matches.map { round ->
                // to be right we should add it as second and first alternatively
                val first = round.first().copy(second = n)
                round.drop(1) + first
            }
        } else {
            // the first match is always i vs i
            matches.map { it.drop(1) }
        }
    }

    private fun generateDivision(teams: List<Team>, matches: List<List<Pair<Int, Int>>>, rounds: List<Round>): List<Match> {
        val shuffledTeams = teams.shuffled()
        return matches.zip(rounds).map { (round, roundEntity) ->
            round.map { (home, away) ->
                val xHome = Random.nextBoolean()
                Match(
                    round = roundEntity,
                    teamX = shuffledTeams[if (xHome) home else away],
                    teamA = shuffledTeams[if (xHome) away else home]
                )
            }
        }.flatten()
    }
}
