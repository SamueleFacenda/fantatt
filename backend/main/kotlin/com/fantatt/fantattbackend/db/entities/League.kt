package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class League (

    val name: String,
    @ManyToOne
    val master: User,
    @ManyToOne
    val season: Season,
    @OneToMany
    val teams: List<Team>?=null,
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    val id: Long?=null,
)