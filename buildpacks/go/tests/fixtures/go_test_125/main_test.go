//go:build heroku

package main

import "testing"

func TestAdd(t *testing.T) {
	got := Add(2, 3)
	if got != 5 {
		t.Errorf("Add(2, 3) = %d; want 5", got)
	}
}

func TestSum(t *testing.T) {
	got := Sum([]int{1, 2, 3, 4})
	if got != 10 {
		t.Errorf("Sum([]int{1,2,3,4}) = %d; want 10", got)
	}
}
