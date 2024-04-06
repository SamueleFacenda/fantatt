package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class SeasonEntity (
    @Column(unique = true, nullable = false)
    val year: Int,
    @OneToMany
    val rounds: MutableList<RoundEntity> = mutableListOf(),
    @OneToMany
    val leagues: MutableList<LeagueEntity> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null
)