package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Player
import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Score
import com.fantatt.fantattbackend.db.entities.Team
import com.fantatt.fantattbackend.db.repos.ScoreRepository

class SimulatedMatch(
    private val teamX: Team,
    private val teamA: Team,
    private val round: Round,
    private val playerScoreComputer: PlayerScoreComputer,
    private val lineupManager: LineupManager,
    private val scoreRepository: ScoreRepository,
    private val gameFormat: GameFormat = GameFormat.SWAYTHLING
) {
    val winner: Team

    init {
        val teamXLineup = getTeamLineUp(teamX, 'X', gameFormat.matches.size)
        val teamALineup = getTeamLineUp(teamA, 'A' , gameFormat.matches.size)

        val result = gameFormat.matches.sumOf { (teamXLetter, teamALetter) ->
            val teamXScore = teamXLineup[teamXLetter]!!
            val teamAScore = teamALineup[teamALetter]!!
            if (teamAScore > teamXScore) 1L else -1
        }
        winner = if (result > 0) teamA else teamX
    }

    private fun getTeamLineUp(team: Team, startLetter: Char, startingSize: Int): Map<String, Int> {
        val lineup = lineupManager
            .getLineup(team, round.index)
            .sortedBy { it.order }
            .map { it.player }
            .map { getPlayerScore(it) }
        return (startLetter..'Z')
            .take(startingSize)
            .map { it.toString() }
            .plus(List(startingSize - lineup.size) { "R" })
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