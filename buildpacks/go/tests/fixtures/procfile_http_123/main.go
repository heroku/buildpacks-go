// +build heroku

package main

import (
	"fmt"
	"os"
	"net/http"
)

func root(w http.ResponseWriter, req *http.Request) {
	fmt.Fprintf(w, "procfile_http_123")
}

func main() {
	port := os.Getenv("PORT")
	if port == "" { port = "8080" }

	http.HandleFunc("/", root)
	http.ListenAndServe(":" + port, nil)
}
