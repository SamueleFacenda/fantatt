package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class User(
    @Column(unique = true, nullable = false)
    val username: String,
    @Column(nullable = false)
    val password: String,
    @Column(unique = true, nullable = false)
    val email: String,
    @OneToMany
    val teams: MutableList<Team> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null
)