package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class User(
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    val id: Long,
    @Column(unique = true, nullable = false)
    val username: String,
    @Column(nullable = false)
    val password: String,
    @Column(unique = true, nullable = false)
    val email: String,
    @OneToMany
    val teams: List<Team>
)