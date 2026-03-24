//go:build heroku

package main

import "testing"

func TestAdd(t *testing.T) {
	got := Add(2, 3)
	if got != 5 {
		t.Errorf("Add(2, 3) = %d; want 5", got)
	}
}
