package com.fantatt.fantattbackend.game

import com.fantatt.fantattbackend.db.entities.Round

interface PlayerScoreComputer {
    fun getPlayerScoreInRound(player: String, round: Round): Int
}