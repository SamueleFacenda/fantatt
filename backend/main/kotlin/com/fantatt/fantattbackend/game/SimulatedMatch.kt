package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Player
import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Score
import com.fantatt.fantattbackend.db.entities.Team
import com.fantatt.fantattbackend.db.repos.ScoreRepository

val GAME_FORMATS = mapOf(
    "SWAYTHLING" to listOf(
        Pair("A", "X"),
        Pair("B", "Y"),
        Pair("C", "Z"),
        Pair("B", "X"),
        Pair("A", "Z"),
        Pair("C", "Y"),
        Pair("B", "Z"),
        Pair("C", "X"),
        Pair("A", "Y")
    ),
    "MINI SWAYTHLING" to listOf( // TODO pareggio
        Pair("A", "X"),
        Pair("B", "Y"),
        Pair("C", "Z"),
        Pair("B", "X"),
        Pair("A", "Z"),
        Pair("C", "Y")
    ),
    "NEW SWAYTHLING " to listOf(
        Pair("A", "X"),
        Pair("B", "Y"),
        Pair("C", "Z"),
        Pair("A", "Y"),
        Pair("B", "X"),
    ),
)

const val GAME_FORMAT = "SWAYTHLING"
val STARTING_SIZE = GAME_FORMATS[GAME_FORMAT]!!.map { it.first }.distinct().size

class SimulatedMatch(
    private val TeamX: Team,
    private val TeamA: Team,
    private val round: Round,
    val playerScoreComputer: PlayerScoreComputer,
    val lineupManager: LineupManager,
    val scoreRepository: ScoreRepository
) {
    val winner: Team

    init {
        val teamXLineup = getTeamLineUp(TeamX, 'X')
        val teamALineup = getTeamLineUp(TeamA, 'A')

        val result = GAME_FORMATS[GAME_FORMAT]!!.sumOf { (teamXLetter, teamALetter) ->
            val teamXScore = teamXLineup[teamXLetter]!!
            val teamAScore = teamALineup[teamALetter]!!
            if (teamAScore > teamXScore) 1L else -1
        }
        winner = if (result > 0) TeamA else TeamX
    }

    private fun getTeamLineUp(team: Team, startLetter: Char): Map<String, Int> {
        val lineup = lineupManager
            .getLineup(team, round.index)
            .sortedBy { it.order }
            .map { it.player }
            .map { getPlayerScore(it) }
        return (startLetter..'Z')
            .take(STARTING_SIZE)
            .map { it.toString() }
            .plus(List(STARTING_SIZE - lineup.size) { "R" })
            .zip(lineup)
            .toMap()
    }

    private fun getPlayerScore(player: Player): Int {
        val score = scoreRepository.findByPlayerAndRound(player, round)
        return score?.score ?: scoreRepository.save(
            Score(
                player = player,
                round = round,
                score = playerScoreComputer.getPlayerScoreInRound(player.name, round)
            )).score
    }
}