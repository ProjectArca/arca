package main

import (
    "encoding/json"
    "fmt"
    "log"
    "net/http"
)

type Hello struct {
    Message string `json:"message"`
}

func helloHandler(w http.ResponseWriter, r *http.Request) {
    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(Hello{Message: "hello"})
}

func main() {
    http.HandleFunc("/", helloHandler)
    addr := ":3000"
    fmt.Println("Go server listening on", addr)
    log.Fatal(http.ListenAndServe(addr, nil))
}
