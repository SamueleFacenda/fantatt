package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class Season (
    @Column(unique = true, nullable = false)
    val year: Int,
    @OneToMany
    val rounds: MutableList<Round> = mutableListOf(),
    @OneToMany
    val leagues: MutableList<League> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null
)