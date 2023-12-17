package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class Season (
    @Column(unique = true, nullable = false)
    val year: Int,
    @OneToMany
    val rounds: List<Round>?=null,
    @OneToMany
    val leagues: List<League>?=null,
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    val id: Long?=null
)