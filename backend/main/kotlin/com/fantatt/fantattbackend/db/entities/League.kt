package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class League (
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    val id: Long,
    val name: String,
    @ManyToOne
    val master: User,
    @ManyToOne
    val season: Season,
    @OneToMany
    val teams: List<Team>
)