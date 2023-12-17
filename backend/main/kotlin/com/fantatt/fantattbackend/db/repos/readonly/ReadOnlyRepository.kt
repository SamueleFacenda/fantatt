package com.fantatt.fantattbackend.db.repos.readonly

import org.springframework.data.repository.NoRepositoryBean
import org.springframework.data.repository.Repository
import java.util.Optional

@NoRepositoryBean
interface ReadOnlyRepository<T, ID>: Repository<T, ID> {
    fun findById(id: ID): T?
}