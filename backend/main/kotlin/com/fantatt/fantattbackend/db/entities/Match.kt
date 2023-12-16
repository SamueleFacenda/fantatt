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
class Match (
    @Id
    @ManyToOne
    val teamA: Team,
    @Id
    @ManyToOne
    val teamX: Team,
    @Id
    @ManyToOne
    val round: Round,
    val scoreA: Int,
    val scoreX: Int,
    val winner: String
)