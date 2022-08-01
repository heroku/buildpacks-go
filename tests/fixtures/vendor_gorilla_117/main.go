package main

import (
	"fmt"
	"os"
	"github.com/gorilla/mux"
	"net/http"
)

func root(w http.ResponseWriter, req *http.Request) {
	fmt.Fprintf(w, "vendor_gorilla_117")
}

func main() {
	port := os.Getenv("PORT")
	if port == "" { port = "8080" }
	r := mux.NewRouter()
	r.HandleFunc("/", root);
	http.Handle("/", r)
	http.ListenAndServe(":" + port, nil)
}
