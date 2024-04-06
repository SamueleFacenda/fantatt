package com.fantatt.fantattbackend.db.repos

import com.fantatt.fantattbackend.db.entities.PlayerEntity
import com.fantatt.fantattbackend.db.entities.RoundEntity
import com.fantatt.fantattbackend.db.entities.ScoreEntity
import com.fantatt.fantattbackend.db.entities.ScoreId
import org.springframework.data.repository.CrudRepository

interface ScoreRepository: CrudRepository<ScoreEntity, ScoreId> {

    fun findByPlayerAndRound(player: PlayerEntity, round: RoundEntity): ScoreEntity?
}