package com.atelier.bookscan

enum class PageLayout {
    SINGLE,
    DOUBLE;

    fun label(): String = when (this) {
        SINGLE -> "1 page"
        DOUBLE -> "2 pages"
    }
}
