package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class Team (
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    val id: Long,
    @ManyToOne
    val league: League,
    @ManyToOne
    val owner: User,
    val name: String,
    val credits: Int,
    @ManyToMany
    val players: List<Player>
)