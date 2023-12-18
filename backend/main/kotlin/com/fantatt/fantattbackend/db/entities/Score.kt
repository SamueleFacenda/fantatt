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
data class Score (
    @Id
    @ManyToOne
    val round: Round,
    @Id
    @ManyToOne
    val player: Player,
    val score: Int
)