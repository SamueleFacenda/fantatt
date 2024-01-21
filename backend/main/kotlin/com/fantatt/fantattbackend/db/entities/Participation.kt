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
data class Participation (
    @Id
    @ManyToOne
    val team: Team,
    @Id
    @ManyToOne
    val player: Player,
    @Id
    @ManyToOne
    val round: Round,
    val order: Int
)