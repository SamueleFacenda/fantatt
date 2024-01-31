package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.matches.MatchRepository
import org.springframework.stereotype.Component

@Component
class SimpleScoreComputer(
    val matchRepository: MatchRepository
): PlayerScoreComputer {
    override fun getPlayerScoreInRound(player: String, round: Round): Int {
        val matches = matchRepository.findAllMatchesByPlayerBetweenDates(player, round.startTime, round.endTime)
        TODO("Not yet implemented")
    }

}