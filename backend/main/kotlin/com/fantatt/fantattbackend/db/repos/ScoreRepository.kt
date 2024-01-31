package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.Player
import com.fantatt.fantattbackend.db.entities.Round
import com.fantatt.fantattbackend.db.entities.Score
import com.fantatt.fantattbackend.db.entities.ScoreId
import org.springframework.data.repository.CrudRepository

interface ScoreRepository: CrudRepository<Score, ScoreId> {

    fun findByPlayerAndRound(player: Player, round: Round): Score?
}