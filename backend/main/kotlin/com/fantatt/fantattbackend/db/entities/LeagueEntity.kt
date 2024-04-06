package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
data class LeagueEntity (
    val name: String,
    @ManyToOne
    val master: UserEntity,
    @ManyToOne
    val season: SeasonEntity,
    val nDivisions: Int = 3,
    @OneToMany
    val societies: MutableList<SocietyEntity> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null,
)