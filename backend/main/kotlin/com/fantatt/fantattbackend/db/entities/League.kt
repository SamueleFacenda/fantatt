package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
data class League (

    val name: String,
    @ManyToOne
    val master: User,
    @ManyToOne
    val season: Season,
    @OneToMany
    val teams: MutableList<Team> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null,
)