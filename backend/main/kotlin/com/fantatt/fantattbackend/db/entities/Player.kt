package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class Player (
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    val id: Long,
    @Column(unique = true, nullable = false)
    val name: String
)