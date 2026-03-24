//go:build heroku

package main

import (
	"fmt"
	"net/http"
	"os"
)

func Add(a, b int) int {
	return a + b
}

func root(w http.ResponseWriter, req *http.Request) {
	fmt.Fprintf(w, "go_test_125")
}

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	http.HandleFunc("/", root)
	http.ListenAndServe(":"+port, nil)
}
