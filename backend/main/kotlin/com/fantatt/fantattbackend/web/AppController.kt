package com.fantatt.fantattbackend.web

import com.fantatt.fantattbackend.db.entities.UserEntity
import com.fantatt.fantattbackend.db.repos.UserRepository
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestBody
import org.springframework.web.bind.annotation.RestController

@RestController
class AppController(
    val userRepository: UserRepository
) {

    @PostMapping("/register")
    fun register(@RequestBody newUser: UserEntity): UserEntity {
        if (userRepository.existsByEmailOrUsername(newUser.email, newUser.username)) {
            throw Exception("User already exists")
        }
        return userRepository.save(newUser)
    }

    @PostMapping("/user/update")
    fun updateUser(@RequestBody updatedUser: UserEntity): UserEntity = TODO()

    // set society name

    // set team name

    // set league name

    // delete user!!

    // delete league

    // get player leagues
}