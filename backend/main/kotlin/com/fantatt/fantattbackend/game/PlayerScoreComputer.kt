package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.RoundEntity

interface PlayerScoreComputer {
    fun getPlayerScoreInRound(player: String, round: RoundEntity): Int
}