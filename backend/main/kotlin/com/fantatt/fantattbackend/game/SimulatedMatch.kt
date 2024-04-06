package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.PlayerEntity
import com.fantatt.fantattbackend.db.entities.RoundEntity
import com.fantatt.fantattbackend.db.entities.ScoreEntity
import com.fantatt.fantattbackend.db.entities.TeamEntity
import com.fantatt.fantattbackend.db.repos.ScoreRepository

class SimulatedMatch(
    private val teamX: TeamEntity,
    private val teamA: TeamEntity,
    private val round: RoundEntity,
    private val playerScoreComputer: PlayerScoreComputer,
    private val lineupManager: LineupManager,
    private val scoreRepository: ScoreRepository,
    private val gameFormat: GameFormat = GameFormat.SWAYTHLING
) {
    val winner: TeamEntity

    init {
        val teamXLineup = getTeamLineUp(teamX, 'X', gameFormat.matches.size)
        val teamALineup = getTeamLineUp(teamA, 'A' , gameFormat.matches.size)

        // TODO update db with results (Match entity)
        val result = gameFormat.matches.sumOf { (teamXLetter, teamALetter) ->
            val teamXScore = teamXLineup[teamXLetter]!!
            val teamAScore = teamALineup[teamALetter]!!
            if (teamAScore > teamXScore) 1L else -1
        }
        winner = if (result > 0) teamA else teamX
    }

    private fun getTeamLineUp(team: TeamEntity, startLetter: Char, startingSize: Int): Map<String, Int> {
        val lineup = lineupManager
            .getLineup(team, round)
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

    private fun getPlayerScore(player: PlayerEntity): Int {
        val score = scoreRepository.findByPlayerAndRound(player, round)
        return score?.score ?: scoreRepository.save(
            ScoreEntity(
                player = player,
                round = round,
                score = playerScoreComputer.getPlayerScoreInRound(player.name, round)
            )).score
    }
}