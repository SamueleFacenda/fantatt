package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
data class SocietyEntity (
    val name: String,
    val credits: Int,
    @ManyToOne
    val league: LeagueEntity,
    @ManyToOne
    val owner: UserEntity,
    val points: Int = 0,
    @ManyToMany
    val players: MutableList<PlayerEntity> = mutableListOf(),
    @OneToMany
    val teams: MutableList<TeamEntity> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null
)