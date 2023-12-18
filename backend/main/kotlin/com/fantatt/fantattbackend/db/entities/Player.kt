package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class Player (
    @Column(unique = true, nullable = false)
    val name: String,
    val points: Int,
    @Id @GeneratedValue val id: Long?=null
)