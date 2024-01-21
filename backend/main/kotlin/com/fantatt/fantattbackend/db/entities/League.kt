package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
data class League (
    val name: String,
    @ManyToOne
    val master: User,
    @ManyToOne
    val season: Season,
    val nDivisions: Int = 3,
    @OneToMany
    val teams: MutableList<Society> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null,
)