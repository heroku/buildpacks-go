package main

import (
	"flag"
	"fmt"
	"log"
	"os"

	"github.com/valyala/fasthttp"
)


func main() {
	flag.Parse()

	port := os.Getenv("PORT")
	if port == "" { port = "8080" }
	addr := ":" + port;

	if err := fasthttp.ListenAndServe(addr, requestHandler); err != nil {
		log.Fatalf("Error in ListenAndServe: %v", err)
	}
}

func requestHandler(ctx *fasthttp.RequestCtx) {
	ctx.SetContentType("text/plain; charset=utf8")
	fmt.Fprintf(ctx, "Hello from vendor_fasthttp_120!")
}
