package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class Season (
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    val id: Long,
    @Column(unique = true, nullable = false)
    val year: Int,
    @OneToMany
    val rounds: List<Round>,
    @OneToMany
    val leagues: List<League>
)