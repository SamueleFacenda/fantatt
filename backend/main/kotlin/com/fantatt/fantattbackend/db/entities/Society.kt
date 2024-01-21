package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
data class Society (
    val name: String,
    val credits: Int,
    @ManyToOne
    val league: League,
    @ManyToOne
    val owner: User,
    val points: Int = 0,
    @ManyToMany
    val players: MutableList<Player> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null
)