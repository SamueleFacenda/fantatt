package com.fantatt.fantattbackend.db.entities

import jakarta.persistence.*

@Entity
@Table(indexes = [Index(columnList = "username", unique = true), Index(columnList = "email", unique = true)])
class UserEntity(
    @Column(unique = true, nullable = false)
    val username: String,
    @Column(nullable = false)
    val password: String,
    @Column(unique = true, nullable = false)
    val email: String,
    @OneToMany
    val societies: MutableList<SocietyEntity> = mutableListOf(),
    @Id @GeneratedValue val id: Long?=null
)