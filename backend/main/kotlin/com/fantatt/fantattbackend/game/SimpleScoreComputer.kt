package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.RoundEntity
import com.fantatt.fantattbackend.db.matches.RealMatchRepository
import org.springframework.stereotype.Component

@Component
class SimpleScoreComputer(
    val realMatchRepository: RealMatchRepository
): PlayerScoreComputer {
    override fun getPlayerScoreInRound(player: String, round: RoundEntity): Int {
        // TODO cache until db update
        val matches = realMatchRepository.findAllMatchesByPlayerBetweenDates(player, round.startTime, round.endTime)
        TODO("Not yet implemented")
    }

}