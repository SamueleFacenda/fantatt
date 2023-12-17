package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class Team (
    val name: String,
    val credits: Int,
    @ManyToOne
    val league: League,
    @ManyToOne
    val owner: User,
    @ManyToMany
    val players: MutableList<Player> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null
)