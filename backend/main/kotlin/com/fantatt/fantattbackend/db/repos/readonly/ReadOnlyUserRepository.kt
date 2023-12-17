package com.fantatt.fantattbackend.db.repos.readonly

import com.fantatt.fantattbackend.db.entities.User
import org.springframework.data.rest.core.annotation.RestResource
// https://docs.spring.io/spring-data/jpa/reference/repositories/query-methods-details.html
@RestResource()
interface ReadOnlyUserRepository: ReadOnlyRepository<User, Long> {
    fun findFirst10ByUsernameContainingIgnoreCase(query: String): List<User>
}