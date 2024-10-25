function sendPostRequest(url, data) {
  return fetch(url, {
    method: "POST",
    body: JSON.stringify(data),
    headers: {
      "Content-Type": "application/json",
    },
  });
}


const Toast = Swal.mixin({
  toast: true,
  position: "bottom-end",
  showConfirmButton: false,
  timer: 3000,
  timerProgressBar: true,
  didOpen: (toast) => {
    toast.onmouseenter = Swal.stopTimer;
    toast.onmouseleave = Swal.resumeTimer;
  }
});

document.addEventListener('keydown', function (event) {
  if (event.key === 'Escape') {
    Swal.clickCancel();
  }
  if (event.key === "Enter") {
    Swal.clickConfirm();
  }

}, true); //use capture so it triggers before bootstrap

document.addEventListener('swiped-left', function (e) {
  calendar.next();
});

document.addEventListener('swiped-right', function (e) {
  calendar.prev();
});

window.mobilecheck = function () {
  var check = false;
  (function (a) { if (/(android|bb\d+|meego).+mobile|avantgo|bada\/|blackberry|blazer|compal|elaine|fennec|hiptop|iemobile|ip(hone|od)|iris|kindle|lge |maemo|midp|mmp|mobile.+firefox|netfront|opera m(ob|in)i|palm( os)?|phone|p(ixi|re)\/|plucker|pocket|psp|series(4|6)0|symbian|treo|up\.(browser|link)|vodafone|wap|windows ce|xda|xiino/i.test(a) || /1207|6310|6590|3gso|4thp|50[1-6]i|770s|802s|a wa|abac|ac(er|oo|s\-)|ai(ko|rn)|al(av|ca|co)|amoi|an(ex|ny|yw)|aptu|ar(ch|go)|as(te|us)|attw|au(di|\-m|r |s )|avan|be(ck|ll|nq)|bi(lb|rd)|bl(ac|az)|br(e|v)w|bumb|bw\-(n|u)|c55\/|capi|ccwa|cdm\-|cell|chtm|cldc|cmd\-|co(mp|nd)|craw|da(it|ll|ng)|dbte|dc\-s|devi|dica|dmob|do(c|p)o|ds(12|\-d)|el(49|ai)|em(l2|ul)|er(ic|k0)|esl8|ez([4-7]0|os|wa|ze)|fetc|fly(\-|_)|g1 u|g560|gene|gf\-5|g\-mo|go(\.w|od)|gr(ad|un)|haie|hcit|hd\-(m|p|t)|hei\-|hi(pt|ta)|hp( i|ip)|hs\-c|ht(c(\-| |_|a|g|p|s|t)|tp)|hu(aw|tc)|i\-(20|go|ma)|i230|iac( |\-|\/)|ibro|idea|ig01|ikom|im1k|inno|ipaq|iris|ja(t|v)a|jbro|jemu|jigs|kddi|keji|kgt( |\/)|klon|kpt |kwc\-|kyo(c|k)|le(no|xi)|lg( g|\/(k|l|u)|50|54|\-[a-w])|libw|lynx|m1\-w|m3ga|m50\/|ma(te|ui|xo)|mc(01|21|ca)|m\-cr|me(rc|ri)|mi(o8|oa|ts)|mmef|mo(01|02|bi|de|do|t(\-| |o|v)|zz)|mt(50|p1|v )|mwbp|mywa|n10[0-2]|n20[2-3]|n30(0|2)|n50(0|2|5)|n7(0(0|1)|10)|ne((c|m)\-|on|tf|wf|wg|wt)|nok(6|i)|nzph|o2im|op(ti|wv)|oran|owg1|p800|pan(a|d|t)|pdxg|pg(13|\-([1-8]|c))|phil|pire|pl(ay|uc)|pn\-2|po(ck|rt|se)|prox|psio|pt\-g|qa\-a|qc(07|12|21|32|60|\-[2-7]|i\-)|qtek|r380|r600|raks|rim9|ro(ve|zo)|s55\/|sa(ge|ma|mm|ms|ny|va)|sc(01|h\-|oo|p\-)|sdk\/|se(c(\-|0|1)|47|mc|nd|ri)|sgh\-|shar|sie(\-|m)|sk\-0|sl(45|id)|sm(al|ar|b3|it|t5)|so(ft|ny)|sp(01|h\-|v\-|v )|sy(01|mb)|t2(18|50)|t6(00|10|18)|ta(gt|lk)|tcl\-|tdg\-|tel(i|m)|tim\-|t\-mo|to(pl|sh)|ts(70|m\-|m3|m5)|tx\-9|up(\.b|g1|si)|utst|v400|v750|veri|vi(rg|te)|vk(40|5[0-3]|\-v)|vm40|voda|vulc|vx(52|53|60|61|70|80|81|83|85|98)|w3c(\-| )|webc|whit|wi(g |nc|nw)|wmlb|wonu|x700|yas\-|your|zeto|zte\-/i.test(a.substr(0, 4))) check = true; })(navigator.userAgent || navigator.vendor || window.opera);
  return check;
};


function switchPages(toPage) {
  let messageDiv = document.getElementById("notices-div");
  let calendarDiv = document.getElementById("calendar-div");
  let messageButton = document.getElementById("notices");
  let calendarButton = document.getElementById("calendar");

  switch (toPage) {
    case "notices":
      messageDiv.removeAttribute("hidden");
      messageButton.setAttribute("class", "active");
      calendarDiv.setAttribute("hidden", "");
      calendarButton.setAttribute("class", "");
      break;
    case "calendar":
      messageDiv.setAttribute("hidden", "");
      messageButton.setAttribute("class", "");
      calendarDiv.removeAttribute("hidden");
      calendarButton.setAttribute("class", "active");
      break;
  }
}

async function logout() {
  let response = await fetch("/api/logout");

  if (response.status !== 200) {
    Toast.fire({
      icon: "error",
      title: "Logout failed",
    });
    return;
  }

  onSignOut();

  Toast.fire({
    icon: "success",
    title: "Logout successful",
  });
}

async function showLoginForm() {
  return new Promise((resolve) => {
    Swal.fire({
      title: 'Login',
      html: `
        <label for="username">Username:</label>
        <input type="text" id="username" class="swal2-input" placeholder="Username" style="margin: 5pt 5pt" required>
        <label for="password">Password:</label>
        <input type="password" id="password" class="swal2-input" placeholder="Password" style="margin: 5pt 5pt" required>
        <p id="nextlogintime"></p>
        `,
      showConfirmButton: true,
      showCancelButton: true,
      padding: '1em',
      confirmButtonText: 'Login',
      confirmButtonColor: '#4BB543',
      allowEnterKey: true,
      cancelButtonText: 'Cancel',
      focusConfirm: false,
      preConfirm: async () => {
        try {
          const response = await sendPostRequest("/api/login", {
            username: document.getElementById("username").value,
            password: document.getElementById("password").value,
          });

          if (response.status === 200) {
            // Handle the successful response here
            Toast.fire({
              icon: "success",
              title: "Login successful",
            });

            const data = await response.json();
            onSignIn(data.user);
            return true;
          } else {
            const errorText = await response.text();
            document.getElementById("nextlogintime").innerHTML = errorText;
            return false;
          }
        } catch (error) {
          Toast.fire({
            icon: "error",
            title: "Login failed",
            text: "Something went wrong",
          });
          return false;
        }
      },
      allowOutsideClick: () => !Swal.isLoading()
    }).then((result) => {
      if (result.dismiss === Swal.DismissReason.cancel) {
        Toast.fire({
          icon: "error",
          title: "Login cancelled",
        });
        resolve(false);
      }
    });
  });
}

async function loadEvents(_, successCallback, failureCallback) {
  try {
    let response = await fetch("/api/book/events");
    let events = await response.json();

    events = events.map((event) => {
      event.start = new Date(event.start);
      event.end = new Date(event.end);
      if (event.owner == room) {
        event.title = "You, " + event.title.split(" ").slice(2).join(" ");
        event.editable = true;
      }
      return event;
    });
    successCallback(events);
  } catch (error) {
    failureCallback(error);
  }
}

function onResize(info) {
  let start = calendar.formatIso(info.event.start).slice(0, -6);
  let end = calendar.formatIso(info.event.end).slice(0, -6);
  let id = parseInt(info.event.id, 10);
  console.log(start, end, id);

  Swal.fire({
    title: 'Reschedule Booking',
    html: `
      <label for="start">New Start Time:</label>
      <input type="datetime-local" id="start" name="start" value="${start}" required>
      <br>
      <label for="end">New End Time:</label>
      <input type="datetime-local" id="end" name="end" value="${end}" required>
    `,
    showCancelButton: true,
    confirmButtonText: 'Reschedule',
    preConfirm: async () => {
      const start = rfc3339(document.getElementById('start').value);
      const end = rfc3339(document.getElementById('end').value);
      reschedule(start, end, id);
    }
  });

}

function reschedule(start_str, end_str, id) {
  const start = rfc3339(start_str);
  const end = rfc3339(end_str);
  sendPostRequest("/api/book/secure/change", {
    start_time: start,
    end_time: end,
    id: parseInt(id, 10),
  }).then((response) => {
    if (response.ok) {
      Toast.fire({
        icon: "success",
        title: "Booking rescheduled"
      });
    } else {
      response.text().then((errorText) => {
        Toast.fire({
          icon: "error",
          title: "Booking reschedule failed",
          text: errorText,
        });
      });
    }
    calendar.refetchEvents();
  });
}


document.addEventListener("DOMContentLoaded", function () {
  var calendarEl = document.getElementById("calendar-div");
  calendar = new FullCalendar.Calendar(calendarEl, {
    initialView: window.mobilecheck() ? "timeGridDay" : "month",
    events: loadEvents,
    height: "100%",
    selectable: true,
    selectMirror: true,
    unselectAuto: false,
    eventClick: handle_event_click,
    weekNumbers: true,
    selectMinDistance: 10,
    select: calendarSelect,
    eventResize: onResize,
    eventDrop: onResize,
    customButtons: {
      newBookingButton: {
        text: 'Create Booking',
        click: function () {
          let now = new Date();
          now.setMinutes(now.getMinutes() - now.getTimezoneOffset());
          now.setDate(now.getDate() + 1);
          let start = now.toISOString().slice(0, -8);
          let end = new Date(now.getTime() + 3600000).toISOString().slice(0, -8);
          bookingPopup(start, end);
        }
      }
    },
    headerToolbar: {
      left: 'today newBookingButton',
      center: 'title',
      right: 'month,timeGridWeek,timeGridDay,prev,next'
    },
    views: {
      month: {
        type: 'dayGridMonth',
        buttonText: 'Month',
        dateClick: (info) => { calendar.changeView('timeGridDay', info.date) },
        dayMaxEventRows: 3,
        selectable: false,
        dayMaxEvents: true,
        eventTimeFormat: {
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
      timeGridWeek: {
        type: 'timeGrid',
        allDaySlot: false,
        slotDuration: '00:15:00',
        dateClick: (info) => {
          let start = new Date(info.date.getTime());
          start.setMinutes(start.getMinutes() - start.getTimezoneOffset());
          let end = new Date(info.date.getTime() + 3600000);
          end.setMinutes(end.getMinutes() - end.getTimezoneOffset());

          bookingPopup(start.toISOString().slice(0, -8), end.toISOString().slice(0, -8));
        },
        slotLabelInterval: '01:00',
        buttonText: 'Week',
        nowIndicator: true,
        scrollTime: '14:00:00',
        slotLabelFormat: {
          week: 'numeric',
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
      timeGridDay: {
        type: 'timeGrid',
        allDaySlot: false,
        slotDuration: '00:15:00',
        dateClick: (info) => {
          let start = new Date(info.date.getTime());
          start.setMinutes(start.getMinutes() - start.getTimezoneOffset());
          let end = new Date(info.date.getTime() + 3600000);
          end.setMinutes(end.getMinutes() - end.getTimezoneOffset());

          bookingPopup(start.toISOString().slice(0, -8), end.toISOString().slice(0, -8));
        },
        slotLabelInterval: '01:00',
        buttonText: 'Day',
        nowIndicator: true,
        scrollTime: '14:00:00',
        slotLabelFormat: {
          hour: 'numeric',
          minute: '2-digit',
          omitZeroMinute: true,
        }
      },
    },
    firstDay: 1,
    locale: "dk",
  });
  calendar.render();
});


async function calendarSelect(info) {
  let start = calendar.formatIso(info.start).slice(0, -6);
  let end = calendar.formatIso(info.end).slice(0, -6);

  bookingPopup(start, end);
}

async function bookingPopup(start, end) {
  await Swal.fire({
    title: 'Select Time',
    html: `
    <label for="resources-dropdown">Select Resource:</label>
    <select id="resources-dropdown" class="resources-dropdown" multiple="multiple">
    </select>
    <br>
    <label for="start">Start Time:</label>
    <input type="datetime-local" id="start" name="start" value="${start}" required>
    <br>
    <label for="end">End Time:</label>
    <input type="datetime-local" id="end" name="end" value="${end}" required>
    `,
    showCancelButton: true,
    confirmButtonText: 'Confirm',
    confirmButtonColor: '#4BB543',
    cancelButtonText: 'Cancel',
    focusConfirm: false,
    didOpen: async function () {
      $('.resources-dropdown').select2({
        dropdownParent: $('#swal2-html-container'),
        placeholder: "Select resources",
        width: '200pt',
        data: await getResources(start, end),
      });
    }
  }).then(async (result) => {
    if (result.isConfirmed) {
      let start = rfc3339(document.getElementById("start").value);
      let end = rfc3339(document.getElementById("end").value);
      let resources = $('.resources-dropdown').select2('data').map((x) => x.id);
      await newBooking(start, end, resources);
    }
    calendar.unselect();
  });
}

async function newBooking(start, end, resources) {
  sendPostRequest("/api/book/secure/new", {
    start_time: start,
    end_time: end,
    resource_names: resources,
  }).then((response) => {
    if (response.ok) {
      Toast.fire({
        icon: "success",
        title: "Booking successful"
      });
    }
    else if (response.status === 401) {
      response.text().then((errorText) => {
        Toast.fire({
          icon: "error",
          title: "Login required",
        });
      });
    }

    else {
      response.text().then((errorText) => {
        Toast.fire({
          icon: "error",
          title: "Booking failed",
          text: errorText,
        });
      });
    }
    calendar.unselect();
    calendar.refetchEvents();
  });
}

function onSubmit(token) {
  document.getElementById("demo-form").submit();
}

function onSignIn(user) {
  if (logged_in == true) return;
  document.getElementById("name-plate").innerHTML = "Room " + user.room;
  document.getElementById("login").innerHTML = "Logout";
  document.getElementById("login").onclick = logout;
  username = user.username;
  room = user.room;
  logged_in = true;
  calendar.refetchEvents();
}

function onSignOut() {
  if (logged_in == false) return;
  document.getElementById("name-plate").innerHTML = "";
  document.getElementById("login").innerHTML = "Login";
  document.getElementById("login").onclick = showLoginForm;
  logged_in = false;
  room = -1;
  calendar.refetchEvents();
}

async function check_login() {
  let response = await fetch("/api/login");
  if (response.status === 202) {
    const data = await response.json();
    onSignIn(data.user);
  } else if (response.status === 200) { // 200 means not logged in
    onSignOut();
  }
}

async function handle_event_click(info) {
  //first check that the event is owned by the user
  let owned = (info.event.extendedProps.owner == room);
  let confirmed = false;
  if (owned && new Date(info.event.start) > new Date()) {
    Swal.fire({
      titleText: info.event.title.split(" ").slice(1).join(" "),
      html: `
        <label for="start">Start Time:</label>
        <input type="datetime-local" id="start" name="start" value="${info.event.startStr.slice(0, -6)}" required>
        <br>
        <label for="end">End Time:</label>
        <input type="datetime-local" id="end" name="end" value="${info.event.endStr.slice(0, -6)}" required>
      `,
      showCancelButton: true,
      confirmButtonText: 'Reschedule',
      confirmButtonColor: '#4BB543',
      showDenyButton: true,
      denyButtonText: confirmed ? 'Are you sure?' : 'Delete',
      denyButtonColor: 'red',
      preDeny: () => {
        // confirm deletion
        if (!confirmed) {
          confirmed = true;
          Swal.getDenyButton().textContent = "Are you sure?"
          return false;
        }
        return true;
      }
    }).then((result) => {
      if (result.isConfirmed) {

        //assert new start and end times are in the future

        if (new Date(document.getElementById('start').value) < new Date()) {
          Toast.fire('Error', 'Start time must be in the future', 'error');
          return;
        }

        if (new Date(document.getElementById('end').value) < new Date()) {
          Toast.fire('Error', 'End time must be in the future', 'error');
          return;
        }

        //Other checks will be done server-side

        const start = rfc3339(document.getElementById('start').value);
        const end = rfc3339(document.getElementById('end').value);
        reschedule(start, end, info.event.id);
      } else if (result.isDenied) {

        sendPostRequest("/api/book/secure/delete", {
          id: parseInt(info.event.id, 10),
        }).then((response) => {
          if (response.ok) {
            Toast.fire('Success', 'Booking deleted successfully', 'success');
            calendar.refetchEvents();
          } else {
            response.text().then((errorText) => {
              Toast.fire('Error', 'Booking delete failed: ' + errorText, 'error');
            });
          }
        })
      }
    });
  } else {
    Swal.fire({
      title: info.event.title,
      html: `
        <label for="start">Start Time:</label>
        <input type="datetime-local" id="start" name="start" style="cursor: default;" value="${info.event.startStr.slice(0, -6)}" required disabled>
        <br>
        <label for="end">End Time:</label>
        <input type="datetime-local" id="end" name="end" style="cursor: default;" value="${info.event.endStr.slice(0, -6)}" required disabled>
      `,
      showCancelButton: false,
      confirmButtonText: 'OK',
      confirmButtonColor: '#4BB543'
    });
  }

}

var logged_in = false;
var username = "";
var room = -1;
document.onload = check_login();
setInterval(async function () {
  await check_login();
}, 10000);

async function getResources(start, end) {
  const response = await fetch('api/book/resources');
  const resources = await response.json();
  // return a list of resource name strings

  let resourceNames = [];
  outer:
  for (const [key, value] of Object.entries(resources)) {
    //check disallowed periods

    if (value.disallowed_periods) {
      for (const [period_name, dates] of Object.entries(value.disallowed_periods)) {

        let is_in_range = (start, end, target) => {
          if (start > end) {
            if (target >= start || target <= end) {
              return true;
            }
          } else {
            if (target >= start && target <= end) {
              return true;
            }
          }
          return false;
        }

        let startmonth = parseInt(start.split("-")[1]);
        let startday = parseInt(start.split("-")[2].split("T")[0]);
        let endmonth = parseInt(end.split("-")[1]);
        let endday = parseInt(end.split("-")[2].split("T")[0]);
        if ((is_in_range(dates.start, dates.end, [startmonth + 1, startday])) ||
          (is_in_range(dates.start, dates.end, [endmonth + 1, endday]))) {
          continue outer;
        }

      }
    }


    resourceNames.push({ id: key, text: value.name });
  }
  return resourceNames;
}

function isoToDate(iso) {
  //2024-05-29T18:00:00
  let date = new Date();
  date.setFullYear(parseInt(iso.slice(0, 4)));
  date.setMonth(parseInt(iso.slice(5, 7)) - 1);
  date.setDate(parseInt(iso.slice(8, 10)));
  date.setHours(parseInt(iso.slice(11, 13)));
  date.setMinutes(parseInt(iso.slice(14, 16)));
  return date;
}

function rfc3339(d) {
  var d = new Date(d);
  function pad(n) {
    return n < 10 ? "0" + n : n;
  }

  function timezoneOffset(offset) {
    var sign;
    if (offset === 0) {
      return "Z";
    }
    sign = (offset > 0) ? "-" : "+";
    offset = Math.abs(offset);
    return sign + pad(Math.floor(offset / 60)) + ":" + pad(offset % 60);
  }

  return d.getFullYear() + "-" +
    pad(d.getMonth() + 1) + "-" +
    pad(d.getDate()) + "T" +
    pad(d.getHours()) + ":" +
    pad(d.getMinutes()) + ":" +
    pad(d.getSeconds()) +
    timezoneOffset(d.getTimezoneOffset());
}

$(document).ready(function () {
  $('.resources-dropdown').select2({
    dropdownParent: $('#create-booking-dialog'),
    placeholder: "Select resources",
    width: 'resolve'
  });
});