package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*
import java.io.Serializable

class ParticipationId (
    val team: Long,
    val player: Long,
    val round: Long
): Serializable

@Entity
@IdClass(ParticipationId::class)
data class ParticipationEntity (
    @Id
    @ManyToOne
    val team: TeamEntity,
    @Id
    @ManyToOne
    val player: PlayerEntity,
    @Id
    @ManyToOne
    val round: RoundEntity,
    val order: Int
)