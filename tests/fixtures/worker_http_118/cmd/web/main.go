package main

import (
	"fmt"
	"os"
	"net/http"
)

func root(w http.ResponseWriter, req *http.Request) {
	fmt.Fprintf(w, "worker_http_118")
}

func main() {
	port := os.Getenv("PORT")
	if port == "" { port = "8080" }

	http.HandleFunc("/", root)
	http.ListenAndServe(":" + port, nil)
}
