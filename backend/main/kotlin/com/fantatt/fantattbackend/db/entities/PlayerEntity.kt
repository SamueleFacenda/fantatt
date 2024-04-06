package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
class PlayerEntity (
    @Column(unique = true, nullable = false)
    val name: String,
    val points: Int,
    @ManyToMany
    val societies: MutableList<SocietyEntity> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null
)