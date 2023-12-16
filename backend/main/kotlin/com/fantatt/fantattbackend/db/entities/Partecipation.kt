package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*
import java.io.Serializable

@Embeddable
class PartecipationId (
    val teamId: Long,
    val playerId: Long,
    val roundId: Long
): Serializable

@Entity
class Partecipation (
    @EmbeddedId
    val id: PartecipationId,
    @ManyToOne
    val team: Team,
    @ManyToOne
    val player: Player,
    @ManyToOne
    val round: Round,
    val score: Int,
    val order: Int,
)