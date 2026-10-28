// Development-only JSON-lines reference driver. Never shipped with the Rust library.
package main

import (
    "encoding/json"
    "fmt"
    "io"
    "os"

    "github.com/tim-riep/go-xslt/engine"
)

type request struct {
    Operation string `json:"operation"`
    engine.Request
}

func main() {
    decoder := json.NewDecoder(os.Stdin)
    encoder := json.NewEncoder(os.Stdout)
    for {
        var req request
        if err := decoder.Decode(&req); err != nil {
            if err == io.EOF { return }
            fmt.Fprintln(os.Stderr, err)
            os.Exit(1)
        }
        var response any
        switch req.Operation {
        case "compile":
            _, diags := engine.Compile(req.Request)
            response = struct { Diagnostics []engine.Diagnostic `json:"diagnostics"` }{diags}
        case "", "transform":
            response = engine.Transform(req.Request)
        default:
            fmt.Fprintln(os.Stderr, "unknown operation:", req.Operation)
            os.Exit(1)
        }
        if err := encoder.Encode(response); err != nil {
            fmt.Fprintln(os.Stderr, err)
            os.Exit(1)
        }
    }
}
