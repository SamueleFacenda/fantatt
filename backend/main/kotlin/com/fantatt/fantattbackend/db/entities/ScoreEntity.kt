package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.Entity
import jakarta.persistence.Id
import jakarta.persistence.IdClass
import jakarta.persistence.ManyToOne
import java.io.Serializable

class ScoreId(
    val round: Long,
    val player: Long
): Serializable

@Entity
@IdClass(ScoreId::class)
data class ScoreEntity (
    @Id
    @ManyToOne
    val round: RoundEntity,
    @Id
    @ManyToOne
    val player: PlayerEntity,
    val score: Int = -1
)