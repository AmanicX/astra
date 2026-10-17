(function () {
  "use strict";

  var reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* scroll reveal */
  var reveals = document.querySelectorAll("[data-reveal]");
  if (!("IntersectionObserver" in window) || reduce) {
    reveals.forEach(function (el) {
      el.classList.add("is-in");
    });
  } else {
    var io = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            entry.target.classList.add("is-in");
            io.unobserve(entry.target);
          }
        });
      },
      { rootMargin: "0px 0px -8% 0px", threshold: 0.12 }
    );
    reveals.forEach(function (el) {
      io.observe(el);
    });
  }

  /* mobile navigation */
  var toggle = document.querySelector(".nav-toggle");
  var mobile = document.getElementById("mobile-nav");
  if (toggle && mobile) {
    toggle.addEventListener("click", function () {
      var open = toggle.getAttribute("aria-expanded") === "true";
      toggle.setAttribute("aria-expanded", open ? "false" : "true");
      toggle.setAttribute("aria-label", open ? "Open menu" : "Close menu");
      mobile.hidden = open;
    });
    mobile.addEventListener("click", function (event) {
      if (event.target.closest("a")) {
        toggle.setAttribute("aria-expanded", "false");
        toggle.setAttribute("aria-label", "Open menu");
        mobile.hidden = true;
      }
    });
  }

  /* approval in the hero fragment */
  var approval = document.querySelector("[data-approval]");
  if (approval) {
    var result = approval.querySelector(".approval-result");
    var approve = approval.querySelector("[data-approve]");
    var deny = approval.querySelector("[data-deny]");

    function resolve(next, message) {
      approval.classList.add("is-resolved");
      if (result) result.textContent = message;
      if (approve) approve.disabled = true;
      if (deny) deny.disabled = true;
      approval.dataset.state = next;
    }

    if (approve) {
      approve.addEventListener("click", function () {
        resolve("approved", "Approved. Capturing once, then describing the screen.");
      });
    }
    if (deny) {
      deny.addEventListener("click", function () {
        resolve("denied", "Denied. Astra will ask for another way to help.");
      });
    }
  }

  /* three state turn */
  var liveRow = document.querySelector(".row-frag.live");
  if (liveRow) {
    var runApprove = liveRow.querySelector("[data-run]");
    var runDeny = liveRow.querySelector("[data-run-deny]");
    function closeRow(message) {
      liveRow.classList.remove("live");
      liveRow.classList.add("done");
      liveRow.style.borderColor = "rgba(63, 107, 82, 0.4)";
      var actions = liveRow.querySelector(".row-actions");
      if (actions) actions.remove();
      var check = document.createElement("span");
      check.className = "row-check";
      check.textContent = message;
      liveRow.appendChild(check);
      var chip = liveRow.querySelector(".chip");
      if (chip) {
        chip.className = "chip chip-low";
        chip.textContent = "ok";
      }
    }
    if (runApprove) {
      runApprove.addEventListener("click", function () {
        closeRow("written, backup kept");
      });
    }
    if (runDeny) {
      runDeny.addEventListener("click", function () {
        closeRow("denied, no change");
      });
    }
  }

  /* segmented control in the closing dialog */
  document.querySelectorAll(".segmented").forEach(function (group) {
    group.addEventListener("click", function (event) {
      var button = event.target.closest(".seg");
      if (!button) return;
      group.querySelectorAll(".seg").forEach(function (sibling) {
        sibling.classList.remove("is-on");
      });
      button.classList.add("is-on");
    });
  });

  /* keep one faq tile open */
  document.querySelectorAll('details[name="faq"]').forEach(function (tile) {
    tile.addEventListener("toggle", function () {
      if (!tile.open) return;
      document.querySelectorAll('details[name="faq"]').forEach(function (other) {
        if (other !== tile) other.open = false;
      });
    });
  });
})();
