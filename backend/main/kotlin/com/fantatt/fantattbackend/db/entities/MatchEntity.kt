package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*
import java.io.Serializable

class MatchId (
    val teamA: Long,
    val teamX: Long,
    val round: Long
): Serializable

@Entity
@IdClass(MatchId::class)
class MatchEntity (
    @Id
    @ManyToOne
    val teamA: TeamEntity,
    @Id
    @ManyToOne
    val teamX: TeamEntity,
    @Id
    @ManyToOne
    val round: RoundEntity,
    @ManyToOne
    val winner: TeamEntity?=null,
    val scoreA: Int = 0,
    val scoreX: Int = 0
)